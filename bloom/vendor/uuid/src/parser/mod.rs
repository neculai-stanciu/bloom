// Copyright 2013-2014 The Rust Project Developers.
// Copyright 2018 The Uuid Project Developers.
//
// See the COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Uuid`] parsing constructs and utilities.
//!
//! [`Uuid`]: ../struct.Uuid.html

pub(crate) mod error;
pub(crate) use self::error::Error;

use crate::{adapter, Uuid};

/// Check if the length matches any of the given criteria lengths.
fn len_matches_any(len: usize, crits: &[usize]) -> bool {
    for crit in crits {
        if len == *crit {
            return true;
        }
    }

    false
}

/// Check if the length matches any criteria lengths in the given range
/// (inclusive).
#[allow(dead_code)]
fn len_matches_range(len: usize, min: usize, max: usize) -> bool {
    for crit in min..(max + 1) {
        if len == crit {
            return true;
        }
    }

    false
}

// Accumulated length of each hyphenated group in hex digits.
const ACC_GROUP_LENS: [usize; 5] = [8, 12, 16, 20, 32];

// Length of each hyphenated group in hex digits.
const GROUP_LENS: [usize; 5] = [8, 4, 4, 4, 12];

impl Uuid {
    /// Parses a `Uuid` from a string of hexadecimal digits with optional
    /// hyphens.
    ///
    /// Any of the formats generated by this module (simple, hyphenated, urn)
    /// are supported by this parsing function.
    pub fn parse_str(mut input: &str) -> Result<Uuid, crate::Error> {
        // Ensure length is valid for any of the supported formats
        let len = input.len();

        if len == adapter::Urn::LENGTH && input.starts_with("urn:uuid:") {
            input = &input[9..];
        } else if !len_matches_any(
            len,
            &[adapter::Hyphenated::LENGTH, adapter::Simple::LENGTH],
        ) {
            Err(Error::InvalidLength {
                expected: error::ExpectedLength::Any(&[
                    adapter::Hyphenated::LENGTH,
                    adapter::Simple::LENGTH,
                ]),
                found: len,
            })?;
        }

        // `digit` counts only hexadecimal digits, `i_char` counts all chars.
        let mut digit = 0;
        let mut group = 0;
        let mut acc = 0;
        let mut buffer = [0u8; 16];

        for (i_char, chr) in input.bytes().enumerate() {
            if digit as usize >= adapter::Simple::LENGTH && group != 4 {
                if group == 0 {
                    Err(Error::InvalidLength {
                        expected: error::ExpectedLength::Any(&[
                            adapter::Hyphenated::LENGTH,
                            adapter::Simple::LENGTH,
                        ]),
                        found: len,
                    })?;
                }

                Err(Error::InvalidGroupCount {
                    expected: error::ExpectedLength::Any(&[1, 5]),
                    found: group + 1,
                })?;
            }

            if digit % 2 == 0 {
                // First digit of the byte.
                match chr {
                    // Calulate upper half.
                    b'0'..=b'9' => acc = chr - b'0',
                    b'a'..=b'f' => acc = chr - b'a' + 10,
                    b'A'..=b'F' => acc = chr - b'A' + 10,
                    // Found a group delimiter
                    b'-' => {
                        // TODO: remove the u8 cast
                        // BODY: this only needed until we switch to
                        //       ParseError
                        if ACC_GROUP_LENS[group] as u8 != digit {
                            // Calculate how many digits this group consists of
                            // in the input.
                            let found = if group > 0 {
                                // TODO: remove the u8 cast
                                // BODY: this only needed until we switch to
                                //       ParseError
                                digit - ACC_GROUP_LENS[group - 1] as u8
                            } else {
                                digit
                            };

                            Err(Error::InvalidGroupLength {
                                expected: error::ExpectedLength::Exact(
                                    GROUP_LENS[group],
                                ),
                                found: found as usize,
                                group,
                            })?;
                        }
                        // Next group, decrement digit, it is incremented again
                        // at the bottom.
                        group += 1;
                        digit -= 1;
                    }
                    _ => {
                        Err(Error::InvalidCharacter {
                            expected: "0123456789abcdefABCDEF-",
                            found: input[i_char..].chars().next().unwrap(),
                            index: i_char,
                            urn: error::UrnPrefix::Optional,
                        })?;
                    }
                }
            } else {
                // Second digit of the byte, shift the upper half.
                acc *= 16;
                match chr {
                    b'0'..=b'9' => acc += chr - b'0',
                    b'a'..=b'f' => acc += chr - b'a' + 10,
                    b'A'..=b'F' => acc += chr - b'A' + 10,
                    b'-' => {
                        // The byte isn't complete yet.
                        let found = if group > 0 {
                            // TODO: remove the u8 cast
                            // BODY: this only needed until we switch to
                            //       ParseError
                            digit - ACC_GROUP_LENS[group - 1] as u8
                        } else {
                            digit
                        };

                        Err(Error::InvalidGroupLength {
                            expected: error::ExpectedLength::Exact(
                                GROUP_LENS[group],
                            ),
                            found: found as usize,
                            group,
                        })?;
                    }
                    _ => {
                        Err(Error::InvalidCharacter {
                            expected: "0123456789abcdefABCDEF-",
                            found: input[i_char..].chars().next().unwrap(),
                            index: i_char,
                            urn: error::UrnPrefix::Optional,
                        })?;
                    }
                }
                buffer[(digit / 2) as usize] = acc;
            }
            digit += 1;
        }

        // Now check the last group.
        // TODO: remove the u8 cast
        // BODY: this only needed until we switch to
        //       ParseError
        if ACC_GROUP_LENS[4] as u8 != digit {
            Err(Error::InvalidGroupLength {
                expected: error::ExpectedLength::Exact(GROUP_LENS[4]),
                found: (digit as usize - ACC_GROUP_LENS[3]),
                group,
            })?;
        }

        Ok(Uuid::from_bytes(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adapter, std::string::ToString, test_util};

    #[test]
    fn test_parse_uuid_v4() {
        const EXPECTED_UUID_LENGTHS: error::ExpectedLength =
            error::ExpectedLength::Any(&[
                adapter::Hyphenated::LENGTH,
                adapter::Simple::LENGTH,
            ]);

        const EXPECTED_GROUP_COUNTS: error::ExpectedLength =
            error::ExpectedLength::Any(&[1, 5]);

        const EXPECTED_CHARS: &'static str = "0123456789abcdefABCDEF-";

        // Invalid
        assert_eq!(
            Uuid::parse_str("").map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 0,
            })
        );

        assert_eq!(
            Uuid::parse_str("!").map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 1
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E45")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 37,
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-BBF-329BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 35
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-BGBF-329BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidCharacter {
                expected: EXPECTED_CHARS,
                found: 'G',
                index: 20,
                urn: error::UrnPrefix::Optional,
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2F4faaFB6BFF329BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupCount {
                expected: EXPECTED_GROUP_COUNTS,
                found: 2
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faaFB6BFF329BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupCount {
                expected: EXPECTED_GROUP_COUNTS,
                found: 3,
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BFF329BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupCount {
                expected: EXPECTED_GROUP_COUNTS,
                found: 4,
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 18,
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faaXB6BFF329BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidCharacter {
                expected: EXPECTED_CHARS,
                found: 'X',
                index: 18,
                urn: error::UrnPrefix::Optional,
            })
        );

        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB-24fa-eB6BFF32-BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupLength {
                expected: error::ExpectedLength::Exact(4),
                found: 3,
                group: 1,
            })
        );
        // (group, found, expecting)
        //
        assert_eq!(
            Uuid::parse_str("01020304-1112-2122-3132-41424344")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupLength {
                expected: error::ExpectedLength::Exact(12),
                found: 8,
                group: 4,
            })
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 31,
            })
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c88")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 33,
            })
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0cg8")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 33,
            })
        );

        assert_eq!(
            Uuid::parse_str("67e5504410b1426%9247bb680e5fe0c8")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidCharacter {
                expected: EXPECTED_CHARS,
                found: '%',
                index: 15,
                urn: error::UrnPrefix::Optional,
            })
        );

        assert_eq!(
            Uuid::parse_str("231231212212423424324323477343246663")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 36,
            })
        );

        // Valid
        assert!(Uuid::parse_str("00000000000000000000000000000000").is_ok());
        assert!(Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E4").is_ok());
        assert!(Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c8").is_ok());
        assert!(Uuid::parse_str("01020304-1112-2122-3132-414243444546").is_ok());
        assert!(Uuid::parse_str(
            "urn:uuid:67e55044-10b1-426f-9247-bb680e5fe0c8"
        )
        .is_ok());

        // Nil
        let nil = Uuid::nil();
        assert_eq!(
            Uuid::parse_str("00000000000000000000000000000000").unwrap(),
            nil
        );
        assert_eq!(
            Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(),
            nil
        );

        // Round-trip
        let uuid_orig = test_util::new();
        let orig_str = uuid_orig.to_string();
        let uuid_out = Uuid::parse_str(&orig_str).unwrap();
        assert_eq!(uuid_orig, uuid_out);

        // Test error reporting
        assert_eq!(
            Uuid::parse_str("67e5504410b1426f9247bb680e5fe0c")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidLength {
                expected: EXPECTED_UUID_LENGTHS,
                found: 31,
            })
        );
        assert_eq!(
            Uuid::parse_str("67e550X410b1426f9247bb680e5fe0cd")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidCharacter {
                expected: EXPECTED_CHARS,
                found: 'X',
                index: 6,
                urn: error::UrnPrefix::Optional,
            })
        );
        assert_eq!(
            Uuid::parse_str("67e550-4105b1426f9247bb680e5fe0c")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupLength {
                expected: error::ExpectedLength::Exact(8),
                found: 6,
                group: 0,
            })
        );
        assert_eq!(
            Uuid::parse_str("F9168C5E-CEB2-4faa-B6BF1-02BF39FA1E4")
                .map_err(crate::Error::expect_parser),
            Err(Error::InvalidGroupLength {
                expected: error::ExpectedLength::Exact(4),
                found: 5,
                group: 3,
            })
        );
    }
}