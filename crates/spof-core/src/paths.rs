//! Resolução pura de diretórios de dados (banco SQLite e cache de recursos).
//!
//! As funções recebem o ambiente por parâmetro para serem testáveis sem tocar
//! em variáveis de ambiente reais.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Diretório base de dados do usuário segundo a spec XDG.
///
/// Usa `XDG_DATA_HOME` ou, na falta, `$HOME/.local/share`. Valores vazios ou
/// relativos são ignorados (a spec XDG exige caminho absoluto), caindo para o
/// próximo candidato. Retorna `None` se nenhum for utilizável.
pub fn xdg_data_dir(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let absolute = |v: Option<OsString>| {
        v.filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    absolute(xdg).or_else(|| absolute(home).map(|h| h.join(".local").join("share")))
}

fn non_empty(v: Option<OsString>) -> Option<PathBuf> {
    v.filter(|s| !s.is_empty()).map(PathBuf::from)
}

/// Diretório de dados do app (`<dados>/0spof`): LOCALAPPDATA, senão XDG.
fn app_data_dir(env: &impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    non_empty(env("LOCALAPPDATA"))
        .or_else(|| xdg_data_dir(env("XDG_DATA_HOME"), env("HOME")))
        .map(|d| d.join("0spof"))
}

/// Diretório do cache de recursos baixados.
///
/// Precedência: `SPOF_CACHE_DIR`, `LOCALAPPDATA`, dados XDG; por fim um
/// diretório relativo (último recurso).
pub fn resolve_cache_dir(env: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    if let Some(custom) = env("SPOF_CACHE_DIR") {
        return PathBuf::from(custom);
    }
    match app_data_dir(&env) {
        Some(dir) => dir.join("resources"),
        None => PathBuf::from("0spof_cache"),
    }
}

/// Caminho do banco e se o diretório pai é gerenciado pelo app (e portanto
/// deve ser criado).
fn resolve_db_path_inner(
    env: &impl Fn(&str) -> Option<OsString>,
    exists: &impl Fn(&Path) -> bool,
) -> (PathBuf, bool) {
    if let Some(custom) = env("SPOF_DB_PATH") {
        return (PathBuf::from(custom), false);
    }
    if exists(Path::new("0spof.db")) {
        return (PathBuf::from("0spof.db"), false);
    }
    if exists(Path::new("../../0spof.db")) {
        return (PathBuf::from("../../0spof.db"), false);
    }
    match app_data_dir(env) {
        Some(dir) => (dir.join("0spof.db"), true),
        None => (PathBuf::from("0spof.db"), false),
    }
}

/// Caminho do banco SQLite.
///
/// Ordem: `SPOF_DB_PATH`, `./0spof.db`, `../../0spof.db`, `LOCALAPPDATA`,
/// `XDG_DATA_HOME`, `$HOME/.local/share` e, por fim, `0spof.db` relativo.
pub fn resolve_db_path(
    env: impl Fn(&str) -> Option<OsString>,
    exists: impl Fn(&Path) -> bool,
) -> PathBuf {
    resolve_db_path_inner(&env, &exists).0
}

/// Resolve o caminho do banco com o ambiente real e, quando ele fica no
/// diretório de dados do app, cria o diretório (modo 0o700 no Unix). Falhas
/// de criação são registradas em stderr.
pub fn resolve_db_path_and_prepare() -> PathBuf {
    let env = |k: &str| std::env::var_os(k);
    let (path, managed) = resolve_db_path_inner(&env, &|p: &Path| p.exists());
    if managed {
        if let Some(parent) = path.parent() {
            if let Err(e) = create_private_dir(parent) {
                eprintln!(
                    "0spof: não foi possível criar o diretório de dados {}: {}",
                    parent.display(),
                    e
                );
            }
        }
    }
    path
}

/// Cria o diretório (e pais) com modo 0o700 no Unix.
pub fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), OsString::from(v)))
            .collect();
        move |k| map.get(k).cloned()
    }

    fn never(_: &Path) -> bool {
        false
    }

    #[test]
    fn xdg_uses_absolute_xdg_first() {
        let d = xdg_data_dir(Some("/x/data".into()), Some("/home/u".into()));
        assert_eq!(d, Some(PathBuf::from("/x/data")));
    }

    #[test]
    fn xdg_ignores_empty_and_relative_xdg() {
        for bad in ["", "relative/dir", "./x"] {
            let d = xdg_data_dir(Some(bad.into()), Some("/home/u".into()));
            assert_eq!(d, Some(PathBuf::from("/home/u/.local/share")), "{bad:?}");
        }
    }

    #[test]
    fn xdg_falls_back_to_home() {
        let d = xdg_data_dir(None, Some("/home/u".into()));
        assert_eq!(d, Some(PathBuf::from("/home/u/.local/share")));
    }

    #[test]
    fn xdg_ignores_empty_and_relative_home() {
        assert_eq!(xdg_data_dir(None, Some("".into())), None);
        assert_eq!(xdg_data_dir(None, Some("home/u".into())), None);
        assert_eq!(xdg_data_dir(Some("".into()), Some("rel".into())), None);
        assert_eq!(xdg_data_dir(None, None), None);
    }

    #[test]
    fn db_custom_path_wins() {
        let env = env_of(&[("SPOF_DB_PATH", "/tmp/a.db"), ("HOME", "/home/u")]);
        assert_eq!(resolve_db_path(env, |_| true), PathBuf::from("/tmp/a.db"));
    }

    #[test]
    fn db_prefers_local_then_parent_file() {
        let env = env_of(&[("HOME", "/home/u")]);
        let p = resolve_db_path(&env, |p| p == Path::new("0spof.db"));
        assert_eq!(p, PathBuf::from("0spof.db"));
        let p = resolve_db_path(&env, |p| p == Path::new("../../0spof.db"));
        assert_eq!(p, PathBuf::from("../../0spof.db"));
    }

    #[test]
    fn db_localappdata_before_xdg() {
        let env = env_of(&[
            ("LOCALAPPDATA", "/la"),
            ("XDG_DATA_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(
            resolve_db_path(env, never),
            PathBuf::from("/la/0spof/0spof.db")
        );
    }

    #[test]
    fn db_empty_localappdata_is_skipped() {
        let env = env_of(&[("LOCALAPPDATA", ""), ("XDG_DATA_HOME", "/xdg")]);
        assert_eq!(
            resolve_db_path(env, never),
            PathBuf::from("/xdg/0spof/0spof.db")
        );
    }

    #[test]
    fn db_xdg_then_home_then_relative() {
        let env = env_of(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/u")]);
        assert_eq!(
            resolve_db_path(env, never),
            PathBuf::from("/xdg/0spof/0spof.db")
        );

        let env = env_of(&[("HOME", "/home/u")]);
        assert_eq!(
            resolve_db_path(env, never),
            PathBuf::from("/home/u/.local/share/0spof/0spof.db")
        );

        assert_eq!(
            resolve_db_path(env_of(&[]), never),
            PathBuf::from("0spof.db")
        );
    }

    #[test]
    fn db_ignores_empty_and_relative_xdg_home() {
        let env = env_of(&[("XDG_DATA_HOME", "rel"), ("HOME", "")]);
        assert_eq!(resolve_db_path(env, never), PathBuf::from("0spof.db"));
        let env = env_of(&[("XDG_DATA_HOME", ""), ("HOME", "/home/u")]);
        assert_eq!(
            resolve_db_path(env, never),
            PathBuf::from("/home/u/.local/share/0spof/0spof.db")
        );
    }

    #[test]
    fn cache_precedence() {
        let env = env_of(&[
            ("SPOF_CACHE_DIR", "/c"),
            ("LOCALAPPDATA", "/la"),
            ("HOME", "/h"),
        ]);
        assert_eq!(resolve_cache_dir(env), PathBuf::from("/c"));

        let env = env_of(&[("LOCALAPPDATA", "/la"), ("HOME", "/h")]);
        assert_eq!(resolve_cache_dir(env), PathBuf::from("/la/0spof/resources"));

        let env = env_of(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/h")]);
        assert_eq!(
            resolve_cache_dir(env),
            PathBuf::from("/xdg/0spof/resources")
        );

        let env = env_of(&[("HOME", "/h")]);
        assert_eq!(
            resolve_cache_dir(env),
            PathBuf::from("/h/.local/share/0spof/resources")
        );
    }

    #[test]
    fn cache_falls_back_to_relative_when_nothing_usable() {
        let env = env_of(&[("XDG_DATA_HOME", "rel"), ("HOME", "")]);
        assert_eq!(resolve_cache_dir(env), PathBuf::from("0spof_cache"));
    }

    #[cfg(unix)]
    #[test]
    fn private_dir_is_0700() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("a").join("0spof");
        create_private_dir(&dir).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        // idempotente
        create_private_dir(&dir).unwrap();
    }
}
