use anyhow::Ok;

use crate::{command::GlobElement, protocol::Literal, store::Store};

pub(super) async fn execute(
    store: &mut impl Store,
    pattern: Vec<GlobElement>,
) -> anyhow::Result<Literal> {
    let found: Vec<Literal> = store
        .keys_where(|key| key_matches(key, &pattern))
        .await?
        .into_iter()
        .map(Literal::BulkString)
        .collect();
    Ok(Literal::Array(found))
}

fn key_matches(key: &str, pattern: &[GlobElement]) -> bool {
    let mut pattern_iterator = pattern.iter();
    let mut key_iterator = key.chars();
    loop {
        match (key_iterator.next(), pattern_iterator.next()) {
            (Some(current_char), Some(GlobElement::Exact(expected))) => {
                if &current_char != expected {
                    return false;
                }
            }
            (Some(current_char), Some(GlobElement::AnyString)) => {
                let rest_of_pattern = pattern_iterator.as_slice();
                let mut remaining_key = String::from(current_char);
                remaining_key.push_str(key_iterator.as_str());
                return remaining_key
                    .char_indices()
                    .any(|(i, _)| key_matches(&remaining_key[i..], rest_of_pattern))
                    || key_matches("", rest_of_pattern);
            }
            (None, Some(GlobElement::AnyString)) => {}
            (Some(_), Some(GlobElement::AnyCharacter)) => {}
            (Some(current_char), Some(GlobElement::Set(set))) => {
                if !set.contains(&current_char) {
                    return false;
                }
            }
            (Some(current_char), Some(GlobElement::NegationSet(set))) => {
                if set.contains(&current_char) {
                    return false;
                }
            }
            (Some(current_char), Some(GlobElement::Range(from, to))) => {
                if !(from <= &current_char && &current_char <= to) {
                    return false;
                }
            }
            (Some(_), None) => return false,
            (None, Some(_)) => return false,
            _ => break,
        }
    }
    true
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::key_matches;
    use crate::{
        command::{Command, GlobElement},
        protocol::Literal,
        service::{Executor, test_utils::set},
        store::Store,
        utils::now_ms,
    };
    use GlobElement::*;

    fn matches(key: &str, pattern: Vec<GlobElement>) -> bool {
        key_matches(&key.to_string(), &pattern)
    }

    fn exact(s: &str) -> Vec<GlobElement> {
        s.chars().map(Exact).collect()
    }

    async fn keys_reply(
        executor: &mut Executor<impl Store>,
        pattern: Vec<GlobElement>,
    ) -> Vec<String> {
        let reply = executor
            .process_command_resp(Command::Keys(pattern))
            .await
            .unwrap();
        let Literal::Array(literals) = reply else {
            panic!("expected an array, got {reply:?}");
        };
        let mut keys: Vec<String> = literals
            .into_iter()
            .map(|l| match l {
                Literal::BulkString(s) => s,
                other => panic!("expected a bulk string, got {other:?}"),
            })
            .collect();
        keys.sort();
        keys
    }

    // key_matches

    #[test]
    fn exact_pattern_matches_same_key() {
        assert!(matches("hello", exact("hello")));
    }

    #[test]
    fn exact_pattern_rejects_different_key() {
        assert!(!matches("hallo", exact("hello")));
    }

    #[test]
    fn exact_pattern_rejects_longer_key() {
        assert!(!matches("hello!", exact("hello")));
    }

    #[test]
    fn exact_pattern_rejects_shorter_key() {
        assert!(!matches("hell", exact("hello")));
    }

    #[test]
    fn any_string_matches_everything() {
        assert!(matches("", vec![AnyString]));
        assert!(matches("anything", vec![AnyString]));
    }

    #[test]
    fn trailing_any_string_matches_prefix() {
        assert!(matches("user:1", vec![Exact('u'), Exact('s'), AnyString]));
        assert!(!matches("session", vec![Exact('u'), Exact('s'), AnyString]));
    }

    #[test]
    fn leading_any_string_matches_suffix() {
        assert!(matches("hello", vec![AnyString, Exact('o')]));
        assert!(!matches("help", vec![AnyString, Exact('o')]));
    }

    #[test]
    fn middle_any_string_matches_prefix_and_suffix() {
        let pattern = || vec![Exact('h'), AnyString, Exact('o')];
        assert!(matches("hello", pattern()));
        assert!(matches("ho", pattern()));
        assert!(!matches("hello!", pattern()));
    }

    #[test]
    fn any_string_may_need_to_backtrack() {
        // the first 'a' must be swallowed by the star
        assert!(matches("aab", vec![AnyString, Exact('a'), Exact('b')]));
    }

    #[test]
    fn any_character_matches_exactly_one() {
        assert!(matches(
            "hallo",
            vec![Exact('h'), AnyCharacter, Exact('l'), Exact('l'), Exact('o')]
        ));
        assert!(!matches(
            "hllo",
            vec![Exact('h'), AnyCharacter, Exact('l'), Exact('l'), Exact('o')]
        ));
        assert!(!matches("", vec![AnyCharacter]));
        assert!(!matches("ab", vec![AnyCharacter]));
    }

    #[test]
    fn set_matches_members_only() {
        assert!(matches("a", vec![Set(vec!['a', 'e'])]));
        assert!(matches("e", vec![Set(vec!['a', 'e'])]));
        assert!(!matches("i", vec![Set(vec!['a', 'e'])]));
        assert!(!matches("", vec![Set(vec!['a', 'e'])]));
    }

    #[test]
    fn negation_set_rejects_members() {
        assert!(!matches("e", vec![NegationSet(vec!['e'])]));
        assert!(matches("a", vec![NegationSet(vec!['e'])]));
        assert!(!matches("", vec![NegationSet(vec!['e'])]));
    }

    #[test]
    fn range_is_inclusive() {
        let pattern = || vec![Range('a', 'c')];
        assert!(matches("a", pattern()));
        assert!(matches("b", pattern()));
        assert!(matches("c", pattern()));
        assert!(!matches("d", pattern()));
        assert!(!matches("", pattern()));
    }

    #[test]
    fn empty_pattern_matches_only_empty_key() {
        assert!(matches("", vec![]));
        assert!(!matches("a", vec![]));
    }

    #[test]
    fn multibyte_characters_count_as_one() {
        assert!(matches(
            "héllo",
            vec![Exact('h'), AnyCharacter, Exact('l'), Exact('l'), Exact('o')]
        ));
    }

    // execute

    #[tokio::test]
    async fn keys_returns_matching_keys() {
        let mut executor = Executor::default();
        for key in ["user:1", "user:2", "session:9"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }
        let mut pattern = exact("user:");
        pattern.push(AnyString);

        assert_eq!(
            keys_reply(&mut executor, pattern).await,
            ["user:1", "user:2"]
        );
    }

    #[tokio::test]
    async fn keys_star_returns_all_keys() {
        let mut executor = Executor::default();
        for key in ["a", "b", "c"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }

        assert_eq!(
            keys_reply(&mut executor, vec![AnyString]).await,
            ["a", "b", "c"]
        );
    }

    #[tokio::test]
    async fn keys_without_match_returns_empty_array() {
        let mut executor = Executor::default();
        executor
            .process_command_resp(set("a", "v", None, None, false))
            .await
            .unwrap();

        let reply = executor
            .process_command_resp(Command::Keys(exact("zzz")))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Array(literals) if literals.is_empty());
    }

    #[tokio::test]
    async fn keys_on_empty_store_returns_empty_array() {
        let mut executor = Executor::default();

        let reply = executor
            .process_command_resp(Command::Keys(vec![AnyString]))
            .await
            .unwrap();

        assert_matches!(reply, Literal::Array(literals) if literals.is_empty());
    }

    #[tokio::test]
    async fn keys_skips_deleted_key() {
        let mut executor = Executor::default();
        for key in ["a", "b"] {
            executor
                .process_command_resp(set(key, "v", None, None, false))
                .await
                .unwrap();
        }
        executor
            .process_command_resp(Command::Del(vec!["a".to_string()]))
            .await
            .unwrap();

        assert_eq!(keys_reply(&mut executor, vec![AnyString]).await, ["b"]);
    }

    #[tokio::test]
    async fn keys_skips_expired_key() {
        use crate::command::SetArgumentExpiryRule;

        let mut executor = Executor::default();
        let past = now_ms() - 1_000;
        executor
            .process_command_resp(set(
                "old",
                "v",
                None,
                Some(SetArgumentExpiryRule::ExpiresAtMs(past)),
                false,
            ))
            .await
            .unwrap();
        executor
            .process_command_resp(set("fresh", "v", None, None, false))
            .await
            .unwrap();

        assert_eq!(keys_reply(&mut executor, vec![AnyString]).await, ["fresh"]);
    }
}
