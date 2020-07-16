use std::collections::HashMap;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt};

use crate::store::{StoreError, StorePath};

#[derive(Debug)]
pub struct Derivation {
    pub outputs: HashMap<String, DerivationOutput>,

    pub input_srcs: crate::store::path::StorePaths,
    pub platform: String,
    pub builder: String,

    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,

    pub inputs: HashMap<StorePath, Vec<String>>,
}

impl Derivation {
    pub async fn from_path(
        path: &crate::store::StorePath,
    ) -> Result<Self, crate::store::StoreError> {
        // TODO: take store as input
        let reader = tokio::fs::File::open(format!("/nix/store/{}", path)).await?;
        let reader = tokio::io::BufReader::new(reader);

        Self::from_reader(reader).await
    }

    pub async fn from_reader<T: AsyncReadExt + Unpin>(
        reader: tokio::io::BufReader<T>,
    ) -> Result<Self, StoreError> {
        let mut reader = reader;
        let mut buf = Vec::new();
        let mut ret = Derivation::new();

        reader.read_until('[' as u8, &mut buf).await?;
        if "Derive([".as_bytes() != buf.as_slice() {
            return Err(StoreError::InvalidDerivation {
                msg: "not a derivation".to_string(),
            });
        }

        buf.clear();
        reader.read_until(']' as u8, &mut buf).await?;
        let outputs = String::from_utf8_lossy(&buf);
        let outputs = outputs.trim_matches(']');
        let outputs: Vec<&str> = outputs.split(')').collect();
        for v in outputs {
            let v = v.trim_matches('(');
            let v: Vec<&str> = v.split(",").map(|v| v.trim_matches('"')).collect();
            if v.len() == 1 {
                continue;
            }
            if v.len() != 4 {
                return Err(StoreError::InvalidDerivation {
                    msg: "Invalid output".to_string(),
                });
            }

            let name = v[0].to_string();
            let out = DerivationOutput::new_from_drv(&v[1..])?;
            ret.outputs.insert(name, out);
        }

        // parse the list of inputs
        buf.clear();
        reader.read_until('[' as u8, &mut buf).await?;
        if ",[".as_bytes() != buf.as_slice() {
            return Err(StoreError::InvalidDerivation {
                msg: "not a derivation".to_string(),
            });
        }

        buf.clear();
        loop {
            if reader.read_u8().await? != '(' as u8 {
                return Err(StoreError::InvalidDerivation {
                    msg: "expected (".to_string(),
                });
            }
            buf.clear();
            reader.read_until(')' as u8, &mut buf).await?;
            let val = String::from_utf8_lossy(&buf);
            let val = val.trim_matches(')');
            let val: Vec<&str> = val.splitn(2, ',').collect();
            if val.len() != 2 {
                return Err(StoreError::InvalidDerivation {
                    msg: "not a derivation".to_string(),
                });
            }

            let path = val[0].trim_matches('"');
            let path = &path[11..]; // TODO: use store here

            let outputs = val[1].trim_matches(|c| c == '[' || c == ']');
            let outputs: Vec<String> = outputs
                .split(',')
                .map(|v| v.trim_matches('"').to_string())
                .collect();

            ret.inputs.insert(StorePath::new(path)?, outputs);

            if reader.read_u8().await? != ',' as u8 {
                break;
            }
        }

        buf.clear();
        reader.read_until('[' as u8, &mut buf).await?;
        if buf.as_slice() != ",[".as_bytes() {
            return Err(StoreError::InvalidDerivation {
                msg: "not a derivation".to_string(),
            });
        }
        buf.clear();
        reader.read_until(']' as u8, &mut buf).await?;
        let val = String::from_utf8_lossy(&buf);
        let val = val.trim_matches(']');
        let val: Result<Vec<StorePath>, StoreError> = val
            .split(',')
            .map(|v| v.trim_matches('"'))
            .map(|v| StorePath::new(&v[11..]))
            .collect();
        ret.input_srcs = val?;
        reader.read_u8().await?;

        buf.clear();
        reader.read_until(',' as u8, &mut buf).await?;
        let val = String::from_utf8_lossy(&buf);
        let val = val.trim_matches(|c| c == '"' || c == ',');
        ret.platform = val.to_string();

        buf.clear();
        reader.read_until(',' as u8, &mut buf).await?;
        let val = String::from_utf8_lossy(&buf);
        let val = val.trim_matches(|c| c == '"' || c == ',');
        ret.builder = val.to_string();

        // parse args
        reader.read_u8().await?;
        buf.clear();
        reader.read_until(']' as u8, &mut buf).await?;
        let val = String::from_utf8_lossy(&buf);
        let val = val.trim_matches(']');
        let val: Vec<String> = val
            .split(',')
            .map(|v| v.trim_matches('"'))
            .map(|v| v.to_string())
            .collect();
        ret.args = val;
        reader.read_u16().await?; // ,[

        // parse env vars
        loop {
            if reader.read_u8().await? != '(' as u8 {
                return Err(StoreError::InvalidDerivation {
                    msg: "expected (".to_string(),
                });
            }
            buf.clear();
            reader.read_until(')' as u8, &mut buf).await?;
            let val = String::from_utf8_lossy(&buf);
            let val = val.trim_matches(')');
            let val: Vec<&str> = val.splitn(2, ',').collect();
            if val.len() != 2 {
                return Err(StoreError::InvalidDerivation {
                    msg: "not a derivation".to_string(),
                });
            }

            let name = val[0].trim_matches('"');

            let val = val[1].trim_matches('"');

            ret.env.insert(name.to_string(), val.to_string());

            if reader.read_u8().await? != ',' as u8 {
                break;
            }
        }

        buf.clear();
        reader.read_to_end(&mut buf).await?;
        if buf.as_slice() != ")".as_bytes() {
            return Err(StoreError::InvalidDerivation {
                msg: "expected )".to_string(),
            });
        }

        Ok(ret)
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
}

#[derive(Debug)]
pub struct DerivationOutput {
    path: StorePath,
    hash: Option<String>, // TODO: use Hash type
}

impl DerivationOutput {
    pub fn new_from_drv(vec: &[&str]) -> Result<DerivationOutput, StoreError> {
        let name: &str = vec[0];
        let name = name.get(11..).unwrap();
        Ok(Self {
            path: StorePath::new(name)?,
            hash: None,
        })
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn read_basic_drv() {
        let drv = r#"Derive([("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10","","")],[("/nix/store/130cylf8ms564hb4h7a8jqmdnqaz4xc2-bash-4.4-p23.drv",["out"]),("/nix/store/jwwz66zxkzm7ymcpfs3h26x39kk3rvm6-hello-2.10.tar.gz.drv",["out"]),("/nix/store/v0d85x08ww9xdgghp6my7rc0m3lzkfy4-stdenv-linux.drv",["out"])],["/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],"x86_64-linux","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash",["-e","/nix/store/yigg1q0y7ynnm0mjl60341aad62sngpd-default-builder.sh"],[("buildInputs",""),("builder","/nix/store/yxdxssjvldpx2gh6d9ggv0a9dg1v6z3i-bash-4.4-p23/bin/bash"),("configureFlags",""),("depsBuildBuild",""),("depsBuildBuildPropagated",""),("depsBuildTarget",""),("depsBuildTargetPropagated",""),("depsHostHost",""),("depsHostHostPropagated",""),("depsTargetTarget",""),("depsTargetTargetPropagated",""),("doCheck","1"),("doInstallCheck",""),("name","hello-2.10"),("nativeBuildInputs",""),("out","/nix/store/gfri16c7bbgfjj44c00q4sfw5wb5i5g9-hello-2.10"),("outputs","out"),("patches",""),("pname","hello"),("propagatedBuildInputs",""),("propagatedNativeBuildInputs",""),("src","/nix/store/3x7dwzq014bblazs7kq20p9hyzz0qh8g-hello-2.10.tar.gz"),("stdenv","/nix/store/y4rca6a87l2l49p55m2mpnwndma21mkx-stdenv-linux"),("strictDeps",""),("system","x86_64-linux"),("version","2.10")])"#.as_bytes();
        let drv = tokio::io::BufReader::new(drv);

        let drv = super::Derivation::from_reader(drv).await.unwrap();

        println!("drv: {:?}", drv);
    }
}
