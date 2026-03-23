use crate::error::{AgiError, Result};

/// parsed AGI response from asterisk
#[derive(Debug, Clone)]
pub struct AgiResponse {
    pub code: u16,
    pub result: i32,
    pub data: Option<String>,
    pub endpos: Option<u64>,
}

impl AgiResponse {
    /// parse an AGI response line
    ///
    /// format: `200 result=1 (timeout) endpos=12345`
    /// - `result=X` is always present for 200 responses
    /// - parenthesized data is optional
    /// - `endpos=N` is optional
    /// - error codes (510, 511, 520) default result to -1
    pub fn parse(line: &str) -> Result<Self> {
        let line = line.trim();

        let (code_str, rest) = line
            .split_once(' ')
            .ok_or_else(|| AgiError::InvalidResponse {
                raw: line.to_owned(),
            })?;

        let code: u16 = code_str.parse().map_err(|_| AgiError::InvalidResponse {
            raw: line.to_owned(),
        })?;

        // non-200 responses: treat as error with default result -1
        if code != 200 {
            return Ok(Self {
                code,
                result: -1,
                data: Some(rest.to_owned()),
                endpos: None,
            });
        }

        // parse result=X
        let rest = rest.trim();
        let result_value = if let Some(stripped) = rest.strip_prefix("result=") {
            stripped
        } else {
            return Err(AgiError::InvalidResponse {
                raw: line.to_owned(),
            });
        };

        // extract the numeric result — everything up to the first space or paren
        let result_end = result_value.find([' ', '(']).unwrap_or(result_value.len());
        let result: i32 =
            result_value[..result_end]
                .parse()
                .map_err(|_| AgiError::InvalidResponse {
                    raw: line.to_owned(),
                })?;

        let remainder = result_value[result_end..].trim();

        // extract optional parenthesized data
        let (data, remainder) = if let Some(start) = remainder.find('(') {
            if let Some(end) = remainder.rfind(')') {
                let data_str = &remainder[start + 1..end];
                let after = remainder[end + 1..].trim();
                (Some(data_str.to_owned()), after)
            } else {
                (None, remainder)
            }
        } else {
            (None, remainder)
        };

        // extract optional endpos=N
        let endpos = if let Some(ep_str) = remainder.strip_prefix("endpos=") {
            // endpos= present but empty or non-numeric is a protocol error
            let ep_end = ep_str
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(ep_str.len());
            let digits = &ep_str[..ep_end];
            if digits.is_empty() {
                return Err(AgiError::InvalidResponse {
                    raw: remainder.to_owned(),
                });
            }
            Some(
                digits
                    .parse::<u64>()
                    .map_err(|_| AgiError::InvalidResponse {
                        raw: remainder.to_owned(),
                    })?,
            )
        } else {
            None
        };

        Ok(Self {
            code,
            result,
            data,
            endpos,
        })
    }
}
