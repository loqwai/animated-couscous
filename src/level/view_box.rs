use std::str::FromStr;

#[derive(Clone, Copy)]
pub(crate) struct ViewBox {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Error)]
pub(crate) enum ParseViewBoxError {
    MissingX,
    MissingY,
    MissingWidth,
    MissingHeight,

    /// Only numeric values allowed for "x". Percentages are not yet supported
    InvalidX,
    /// Only numeric values allowed for "y". Percentages are not yet supported
    InvalidY,
    /// Only numeric values allowed for "height". Percentages are not yet supported
    InvalidWidth,
    /// Only numeric values allowed for "width". Percentages are not yet supported
    InvalidHeight,
}

impl FromStr for ViewBox {
    type Err = ParseViewBoxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(4, " ").collect();

        Ok(ViewBox {
            x: parts
                .get(0)
                .ok_or(ParseViewBoxError::MissingX)?
                .parse()
                .or(Err(ParseViewBoxError::InvalidX))?,
            y: parts
                .get(1)
                .ok_or(ParseViewBoxError::MissingY)?
                .parse()
                .or(Err(ParseViewBoxError::InvalidY))?,
            width: parts
                .get(2)
                .ok_or(ParseViewBoxError::MissingWidth)?
                .parse()
                .or(Err(ParseViewBoxError::InvalidWidth))?,
            height: parts
                .get(3)
                .ok_or(ParseViewBoxError::MissingHeight)?
                .parse()
                .or(Err(ParseViewBoxError::InvalidHeight))?,
        })
    }
}
