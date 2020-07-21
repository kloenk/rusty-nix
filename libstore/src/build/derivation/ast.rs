use super::{StoreError, TokType};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Ast {
    pub def: AstNode,
}

impl Ast {
    pub fn from_lexer(lex: Vec<TokType>) -> Result<Self, StoreError> {
        let mut ret = AstNode::Empty;

        let mut it = lex.iter().peekable();

        if let Some(&v) = it.peek() {
            it.next();
            if !(v == &TokType::StartToken) {
                return Err(StoreError::InvalidDerivation {
                    msg: "No StartToken".to_string(),
                });
            }
        }

        if let Some(&v) = it.peek() {
            it.next();
            if !(v == &TokType::LParen) {
                return Err(StoreError::InvalidDerivation {
                    msg: "No top level tuple".to_string(),
                });
            }
            ret = AstNode::Tuple(Self::parse_tuple(&mut it)?),
        }

        Ok(Self { def: ret })
    }

    fn parse_tuple(
        it: &mut std::iter::Peekable<std::slice::Iter<TokType>>,
    ) -> Result<Vec<AstNode>, StoreError> {
        let mut ret = Vec::new();

        while let Some(&t) = it.peek() {
            match t {
                TokType::RParen => {
                    it.next();
                    break;
                }
                TokType::String(v) => {
                    it.next();
                    ret.push(AstNode::String(v.clone()));
                }
                TokType::LBracket => {
                    it.next();
                    let v = Self::parse_array(it)?;
                    ret.push(AstNode::Array(v));
                }
                TokType::Comma => {
                    it.next();
                }

                _ => {
                    return Err(StoreError::InvalidDerivation {
                        msg: format!("did not expect {:?} in tuple", t),
                    });
                }
            }
        }

        Ok(ret)
    }

    fn parse_array(
        it: &mut std::iter::Peekable<std::slice::Iter<TokType>>,
    ) -> Result<Vec<AstNode>, StoreError> {
        let mut ret = Vec::new();

        while let Some(&t) = it.peek() {
            match t {
                TokType::RBracket => {
                    it.next();
                    break;
                }
                TokType::String(v) => {
                    it.next();
                    ret.push(AstNode::String(v.clone()));
                }
                TokType::Comma => {
                    it.next();
                }
                TokType::LParen => {
                    it.next();
                    ret.push(AstNode::Tuple(Self::parse_tuple(it)?));
                }
                _ => {
                    return Err(StoreError::InvalidDerivation {
                        msg: format!("did not expect {:?} in array", t),
                    });
                }
            }
        }

        Ok(ret)
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum AstNode {
    Array(Vec<AstNode>),
    Tuple(Vec<AstNode>),
    String(String),
    Empty,
}

#[cfg(test)]
mod test {
    use super::{Ast, AstNode, StoreError, TokType};

    #[test]
    fn string_array() {
        let tok = vec![
            TokType::StartToken,
            TokType::LParen,
            TokType::LBracket,
            TokType::String("Hello".to_string()),
            TokType::Comma,
            TokType::String("World".to_string()),
            TokType::RBracket,
            TokType::RParen,
        ];

        let ast = Ast::from_lexer(tok).unwrap();

        let ast_exp = AstNode::Tuple(vec![AstNode::Array(vec![
            AstNode::String("Hello".to_string()),
            AstNode::String("World".to_string()),
        ])]);

        assert_eq!(ast.def, ast_exp);

        println!("ast: {:?}", ast);
    }

    #[test]
    /// this test just checks for panics while parsing the drv of hello
    fn lex_and_ast_hello() {
        let drv = r#"Derive([("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10","","")],[("/nix/store/130cylf8ms564hb4h7a8jqmdnqaz4xc2-bash-4.4-p23.drv",["out"]),("/nix/store/jwwz66zxkzm7ymcpfs3h26x39kk3rvm6-hello-2.10.tar.gz.drv",["out"]),("/nix/store/v0d85x08ww9xdgghp6my7rc0m3lzkfy4-stdenv-linux.drv",["out"])],["/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],"x86_64-linux","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash",["-e","/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],[("buildInputs",""),("builder","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash"),("configureFlags",""),("depsBuildBuild",""),("depsBuildBuildPropagated",""),("depsBuildTarget",""),("depsBuildTargetPropagated",""),("depsHostHost",""),("depsHostHostPropagated",""),("depsTargetTarget",""),("depsTargetTargetPropagated",""),("doCheck","1"),("doInstallCheck",""),("name","hello-2.10"),("nativeBuildInputs",""),("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10"),("outputs","out"),("patches",""),("pname","hello"),("propagatedBuildInputs",""),("propagatedNativeBuildInputs",""),("src","/nix/store/3x7dwzq014bblazs7kq20p9hyzz0qh8g-hello-2.10.tar.gz"),("stdenv","/nix/store/y4rca6a87l2l49p55m2mpnwndma21mkx-stdenv-linux"),("strictDeps",""),("system","x86_64-linux"),("version","2.10")])"#;

        let drv = super::TokType::parse(drv).unwrap();

        let drv = Ast::from_lexer(drv).unwrap();

        println!("ast: {:?}", drv);
    }

    #[test]
    /// like lex_and_ast_hello, but for gcc, as that drv is more complicated
    fn lex_and_ast_gcc() {
        let drv = std::fs::read_to_string("./tests/gcc.drv").unwrap();

        let drv = super::TokType::parse(&drv).unwrap();

        let drv = Ast::from_lexer(drv).unwrap();

        println!("ast: {:?}", drv);
    }
}
