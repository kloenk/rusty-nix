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
            ret = AstNode::Tuple(Self::parse_tuple(&mut it)?);
        }

        Ok(Self { def: ret })
    }

    pub fn from_str(str: &str) -> Result<Self, StoreError> {
        let drv = TokType::parse(str)?;
        Self::from_lexer(drv)
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

impl AstNode {
    pub fn to_string(&self) -> Result<String, StoreError> {
        match &self {
            AstNode::String(v) => Ok(v.clone()),
            _ => Err(StoreError::InvalidDerivation {
                msg: "node is not a string".to_string(),
            }),
        }
    }

    pub fn to_array(&self) -> Result<Vec<AstNode>, StoreError> {
        match &self {
            AstNode::Array(v) => Ok(v.clone()),
            _ => Err(StoreError::InvalidDerivation {
                msg: "node is not a array".to_string(),
            }),
        }
    }
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
        let drv = super::super::test::HELLO_DRV;

        let drv = super::TokType::parse(drv).unwrap();

        let drv = Ast::from_lexer(drv).unwrap();

        println!("ast: {:?}", drv);
    }

    #[test]
    /// like lex_and_ast_hello, but for gcc, as that drv is more complicated
    fn lex_and_ast_gcc() {
        let drv_str = std::fs::read_to_string("./tests/gcc.drv").unwrap();

        let drv = super::TokType::parse(&drv_str).unwrap();

        let drv_1 = Ast::from_lexer(drv).unwrap();
        let drv_2 = Ast::from_str(&drv_str).unwrap();

        assert_eq!(drv_1, drv_2);

        println!("ast: {:?}", drv_1);
    }
}
