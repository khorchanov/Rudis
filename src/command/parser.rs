use std::str::FromStr;

use crate::protocol::Literal;

pub(super) fn parse<T>(iterator: &mut impl Iterator<Item = Literal>) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let message = iterator
        .next()
        .ok_or_else(|| anyhow::anyhow!("Protocol Error"))?
        .into_anything()?;
    Ok(message)
}

pub(super) fn parse_opt<T>(
    iterator: &mut impl Iterator<Item = Literal>,
) -> anyhow::Result<Option<T>>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    iterator.next().map(Literal::into_anything).transpose()
}

pub(super) fn parse_vec<T>(iterator: &mut impl Iterator<Item = Literal>) -> anyhow::Result<Vec<T>>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let results = iterator
        .map(Literal::into_anything)
        .collect::<anyhow::Result<Vec<T>>>()?;
    Ok(results)
}
