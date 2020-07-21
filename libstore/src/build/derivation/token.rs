use super::{AsyncRead, AsyncReadExt, StoreError};

use tokio::io::{AsyncSeek, AsyncSeekExt};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokType {
    StartToken, // Derivation
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    Comma,      // ,
    String(String),
}

impl TokType {
    // Just parse a string, DRVs should not be that big
    // TODO: maybe?? change to AsyncRead + AsyncSeek + Unpin??
    pub fn parse(drv: &str) -> Result<Vec<Self>, StoreError> {
        let mut result = Vec::new();

        let mut it = drv.chars().peekable();

        while let Some(&c) = it.peek() {
            match c {
                'D' => {
                    it.next();
                    let mut s = "D".to_string();
                    while let Some(&c) = it.peek() {
                        match c {
                            'e' | 'r' | 'i' | 'v' => {
                                s.push(c);
                                it.next();
                            }
                            _ => break,
                        }
                    }
                    if !(s == "Derive") {
                        return Err(StoreError::InvalidDerivation {
                            msg: format!("Wanted start token 'Derivation', got '{}'", s),
                        });
                    }
                    result.push(TokType::StartToken);
                }
                '(' => {
                    it.next();
                    result.push(TokType::LParen);
                }
                ')' => {
                    it.next();
                    result.push(TokType::RParen);
                }
                '[' => {
                    it.next();
                    result.push(TokType::LBracket);
                }
                ']' => {
                    it.next();
                    result.push(TokType::RBracket);
                }
                ',' => {
                    it.next();
                    result.push(TokType::Comma);
                }
                '"' => {
                    it.next();
                    let mut s = String::new();
                    while let Some(c) = it.next() {
                        match c {
                            '"' => {
                                break;
                            }
                            '\\' => {
                                s.push(c);
                                s.push(it.next().unwrap());
                            }
                            _ => {
                                s.push(c);
                            }
                        }
                    }
                    log::trace!("string: {}", s);
                    result.push(TokType::String(s));
                }
                _ => {
                    return Err(StoreError::InvalidDerivation {
                        msg: format!("Invalid character: {}", c),
                    })
                } // TODO: allow spaces and newlines?
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::TokType;
    #[test]
    fn to_tok_vec() {
        //let drv = r#"Derive([("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10","","")],[("/nix/store/130cylf8ms564hb4h7a8jqmdnqaz4xc2-bash-4.4-p23.drv",["out"]),("/nix/store/jwwz66zxkzm7ymcpfs3h26x39kk3rvm6-hello-2.10.tar.gz.drv",["out"]),("/nix/store/v0d85x08ww9xdgghp6my7rc0m3lzkfy4-stdenv-linux.drv",["out"])],["/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],"x86_64-linux","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash",["-e","/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],[("buildInputs",""),("builder","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash"),("configureFlags",""),("depsBuildBuild",""),("depsBuildBuildPropagated",""),("depsBuildTarget",""),("depsBuildTargetPropagated",""),("depsHostHost",""),("depsHostHostPropagated",""),("depsTargetTarget",""),("depsTargetTargetPropagated",""),("doCheck","1"),("doInstallCheck",""),("name","hello-2.10"),("nativeBuildInputs",""),("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10"),("outputs","out"),("patches",""),("pname","hello"),("propagatedBuildInputs",""),("propagatedNativeBuildInputs",""),("src","/nix/store/3x7dwzq014bblazs7kq20p9hyzz0qh8g-hello-2.10.tar.gz"),("stdenv","/nix/store/y4rca6a87l2l49p55m2mpnwndma21mkx-stdenv-linux"),("strictDeps",""),("system","x86_64-linux"),("version","2.10")])"#;
        let drv = r#"Derive([("out","/nix/store")]"#;

        let drv = TokType::parse(drv).unwrap();

        let drv_exp = vec![
            TokType::StartToken,
            TokType::LParen,
            TokType::LBracket,
            TokType::LParen,
            TokType::String("out".to_string()),
            TokType::Comma,
            TokType::String("/nix/store".to_string()),
            TokType::RParen,
            TokType::RBracket,
        ];

        assert_eq!(drv, drv_exp);
        println!("drv_vec: {:?}", drv);
    }
}
