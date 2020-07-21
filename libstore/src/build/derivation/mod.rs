use std::collections::HashMap;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt};

use crate::store::{Store, StoreError, StorePath};

mod token;
pub use token::TokType;

pub mod ast;
pub use ast::Ast;
use ast::AstNode;

#[derive(Debug)]
pub struct ParsedDerivation {
    pub drv_path: StorePath,
    pub derivation: Derivation,
    // structuredAttrs: Option<Rc<json::Value>>, // TODO: Rc or Arc?
}

impl ParsedDerivation {
    pub fn new(drv_path: StorePath, derivation: Derivation) -> Result<Self, StoreError> {
        if derivation.env.contains_key("__json") {
            unimplemented!("__json attribute in derivation");
        }

        Ok(Self {
            drv_path,
            derivation,
        })
    }

    pub fn get_string_attr(&self, name: &str) -> Option<String> {
        //TODO: structuredAttrs
        self.derivation.env.get(name).map(|v| v.to_string())
    }

    pub fn get_bool_attr(&self, name: &str) -> bool {
        self.get_bool_attr_default(name, false)
    }

    pub fn get_bool_attr_default(&self, name: &str, default: bool) -> bool {
        // TODO: structuredAttrs
        self.get_string_attr(name)
            .map(|v| v == "1")
            .unwrap_or(default)
    }

    pub fn get_strings_attr(&self, name: &str) -> Option<Vec<String>> {
        // TODO: structuredAttrs
        self.get_string_attr(name)
            .map(|v| v.split(' ').map(|v| v.to_string()).collect())
    }

    pub fn get_required_system_features(&self) -> Vec<String> {
        self.get_strings_attr("requiredSystemFeatures")
            .unwrap_or_else(|| Vec::new())
    }

    pub fn can_build_locally(&self) -> bool {
        let settings = crate::CONFIG.read().unwrap();
        let system = settings.system.clone();
        let extra_platform = settings.extra_platforms.clone();
        let features = settings.system_features.clone();
        drop(settings);

        if self.derivation.platform != system
            && !extra_platform.contains(&self.derivation.platform)
            && !self.derivation.is_builtin()
        {
            return false;
        }

        for v in &self.get_required_system_features() {
            if !features.contains(v) {
                return false;
            }
        }

        true
    }

    pub fn will_build_locally(&self) -> bool {
        self.get_bool_attr("preferLocalBuild") && self.can_build_locally()
    }

    pub fn substitutes_allowed(&self) -> bool {
        self.get_bool_attr_default("allowSubstitutes", true)
    }

    pub fn content_addressed(&self) -> bool {
        self.get_string_attr("__contentAddressed").is_some()
    }
}

#[derive(Debug)]
pub struct Derivation {
    pub outputs: HashMap<String, DerivationOutput>,

    pub input_srcs: crate::store::path::StorePaths,
    pub platform: String,
    pub builder: String, // TODO: should this be a store path?

    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,

    pub inputs: HashMap<StorePath, Vec<String>>,
}

impl Derivation {
    pub async fn from_path(
        path: &crate::store::StorePath,
        store: &dyn crate::store::Store,
    ) -> Result<Self, crate::store::StoreError> {
        let drv = tokio::fs::read_to_string(store.print_store_path(path)).await?;

        Self::from_str(&drv, store)
    }

    pub fn from_str(str: &str, store: &dyn Store) -> Result<Self, StoreError> {
        let ast = Ast::from_str(str)?;

        Self::parse_ast(&ast, store)
    }

    pub fn parse_ast(ast: &Ast, store: &dyn Store) -> Result<Self, StoreError> {
        if let AstNode::Tuple(v) = &ast.def {
            let mut ret = Self::new();
            if v.len() > 7 {
                // using > to allow new fields added later to derivations
                return Err(StoreError::InvalidDerivation {
                    msg: format!("Derivation Tuple has {} elements", v.len()),
                });
            }
            if let AstNode::Array(v) = &v[0] {
                ret.outputs = Self::parse_outputs(v, store)?;
            } else {
                return Err(StoreError::InvalidDerivation {
                    msg: "outputs is not an array".to_string(),
                });
            }

            if let AstNode::Array(v) = &v[1] {
                for v in v {
                    if let AstNode::Tuple(v) = v {
                        let path = store.parse_store_path(&v[0].to_string()?)?;
                        let outputs: Result<Vec<String>, StoreError> = v[1]
                            .to_array()?
                            .into_iter()
                            .map(|v| v.to_string())
                            .collect();
                        let outputs = outputs?;
                        ret.inputs.insert(path, outputs);
                    } else {
                        return Err(StoreError::InvalidDerivation {
                            msg: "inputs element is not a tuple".to_string(),
                        });
                    }
                }
            } else {
                return Err(StoreError::InvalidDerivation {
                    msg: "inputs is not an array".to_string(),
                });
            }

            for v in v[2].to_array()? {
                ret.input_srcs
                    .push(store.parse_store_path(&v.to_string()?)?);
            }

            ret.platform = v[3].to_string()?;

            ret.builder = v[4].to_string()?;

            for v in v[5].to_array()? {
                ret.args.push(v.to_string()?);
            }

            for v in v[6].to_array()? {
                match &v {
                    AstNode::Tuple(v) => {
                        if v.len() != 2 {
                            return Err(StoreError::InvalidDerivation {
                                msg: "env variable is not a pair".to_string(),
                            });
                        }
                        ret.env.insert(v[0].to_string()?, v[1].to_string()?);
                    }
                    _ => {
                        return Err(StoreError::InvalidDerivation {
                            msg: "env is not a tuple".to_string(),
                        })
                    }
                }
            }

            return Ok(ret);
        } else {
            return Err(StoreError::InvalidDerivation {
                msg: "Derivation does not contain an Tuple".to_string(),
            });
        }

        unreachable!()
    }

    fn parse_outputs(
        ast: &[ast::AstNode],
        store: &dyn Store,
    ) -> Result<HashMap<String, DerivationOutput>, StoreError> {
        let mut map = HashMap::new();
        for v in ast {
            if let ast::AstNode::Tuple(v) = v {
                let (name, out) = DerivationOutput::from_ast(v, store)?;
                map.insert(name, out);
            } else {
                return Err(StoreError::InvalidDerivation {
                    msg: "output is not a tuple".to_string(),
                });
            }
        }
        Ok(map)
    }

    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            input_srcs: Vec::new(),
            inputs: HashMap::new(),
            platform: String::new(), // TODO: should this default to currentSystem?
            builder: String::new(),  // TODO: should this be a StorePath?
            args: Vec::new(),
            env: HashMap::new(),
        }
    }

    pub fn is_builtin(&self) -> bool {
        self.builder.starts_with("builtin:")
    }
}

#[derive(Debug)]
pub struct DerivationOutput {
    pub path: StorePath,
    pub hash: Option<String>, // TODO: use Hash type
}

impl DerivationOutput {
    pub fn from_ast(ast: &[ast::AstNode], store: &dyn Store) -> Result<(String, Self), StoreError> {
        if ast.len() != 4 {
            return Err(StoreError::InvalidDerivation {
                msg: "output has not 4 elements".to_string(),
            });
        }

        let ret = Self {
            path: store.parse_store_path(&ast[1].to_string()?)?,
            hash: None,
        };

        if ast[2].to_string()? != "" || ast[3].to_string()? != "" {
            log::warn!("output [2..3] is not '\"\"'");
            unreachable!("output [2..3] not \"\"");
        }

        let name = ast[0].to_string()?;
        Ok((name, ret))
    }
}

#[cfg(test)]
mod test {
    pub const HELLO_DRV: &str = r#"Derive([("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10","","")],[("/nix/store/130cylf8ms564hb4h7a8jqmdnqaz4xc2-bash-4.4-p23.drv",["out"]),("/nix/store/jwwz66zxkzm7ymcpfs3h26x39kk3rvm6-hello-2.10.tar.gz.drv",["out"]),("/nix/store/v0d85x08ww9xdgghp6my7rc0m3lzkfy4-stdenv-linux.drv",["out"])],["/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],"x86_64-linux","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash",["-e","/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],[("buildInputs",""),("builder","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash"),("configureFlags",""),("depsBuildBuild",""),("depsBuildBuildPropagated",""),("depsBuildTarget",""),("depsBuildTargetPropagated",""),("depsHostHost",""),("depsHostHostPropagated",""),("depsTargetTarget",""),("depsTargetTargetPropagated",""),("doCheck","1"),("doInstallCheck",""),("name","hello-2.10"),("nativeBuildInputs",""),("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10"),("outputs","out"),("patches",""),("pname","hello"),("propagatedBuildInputs",""),("propagatedNativeBuildInputs",""),("src","/nix/store/3x7dwzq014bblazs7kq20p9hyzz0qh8g-hello-2.10.tar.gz"),("stdenv","/nix/store/y4rca6a87l2l49p55m2mpnwndma21mkx-stdenv-linux"),("strictDeps",""),("system","x86_64-linux"),("version","2.10")])"#;

    #[tokio::test]
    async fn read_basic_drv() {
        let drv = HELLO_DRV;

        let store = std::sync::Arc::new(crate::store::mock_store::MockStore::new());

        //let drv = super::Derivation::from_reader(drv).await.unwrap();
        let drv = super::Derivation::from_str(drv, &store).unwrap();

        println!("drv: {:?}", drv);
    }
}
