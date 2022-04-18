use std::collections::HashMap;
use std::lazy::SyncLazy;

use regex::Regex;

use crate::{Error, Result, TryCollect};

pub(crate) type TransformerFn = (fn(&mut Vec<u8>, Option<isize>), &'static str);

static JS_FUNCTION_REGEX: SyncLazy<Regex> = SyncLazy::new(||
    Regex::new(r"\w+\.(\w+)\(\w,(\d+)\)").unwrap()
);

pub(crate) struct Cipher {
    transform_plan: Vec<String>,
    transform_map: HashMap<String, TransformerFn>,
}

impl Cipher {
    pub(crate) fn from_js(js: &str) -> Result<Self> {
        let transform_plan = get_transform_plan(js)?;

        let (var, _): (&str, &str) = transform_plan
            .get(0)
            .ok_or_else(|| Error::UnexpectedResponse(
                "the provided JavaScript has an empty transform-plan".into()
            ))?
            .split('.')
            .try_collect()
            .ok_or_else(|| Error::UnexpectedResponse(
                "the transform-plan function call contains more then one dot".into()
            ))?;

        let transform_map = get_transform_map(js, var)?;

        Ok(Self {
            transform_plan,
            transform_map,
        })
    }

    pub(crate) fn decrypt_signature(&self, signature: &mut String) -> Result<()> {
        // SAFETY:
        // At the end of the function, `signature` is checked, and, if it's not valid utf-8,
        // completely cleared. So in case, the transformations mess something up, signature
        // will have len 0.
        let signature = unsafe { signature.as_mut_vec() };

        for js_fun_name in self.transform_plan.iter() {
            let (name, argument) = self.parse_function(js_fun_name)?;
            let js_fun = self.transform_map
                .get(name)
                .ok_or_else(|| Error::UnexpectedResponse(format!(
                    "no matching transform function for `{}`", js_fun_name
                ).into()))?
                .0;
            js_fun(signature, argument);
        }

        if std::str::from_utf8(signature).is_err() {
            let err = self.invalid_utf8_err(signature);
            // signature **must** be cleared, it does not contain valid utf-8
            signature.clear();
            return Err(Error::Fatal(err));
        }

        Ok(())
    }

    fn parse_function<'a>(&self, js_func: &'a str) -> Result<(&'a str, Option<isize>)> {
        let (fn_name, fn_arg) = JS_FUNCTION_REGEX
            .captures(js_func)
            .ok_or_else(|| Error::UnexpectedResponse(format!(
                "the JS_FUNCTION_REGEX `{}` did not match the JavaScript function {}",
                *JS_FUNCTION_REGEX, js_func
            ).into()))?
            .iter()
            .skip(1)
            .try_collect()
            .expect("JS_FUNCTION_REGEX must only contain two capture groups");

        match (fn_name, fn_arg) {
            (Some(name), Some(arg)) => Ok((
                name.as_str(),
                Some(
                    arg
                        .as_str()
                        .parse::<isize>()
                        .map_err(|_| Error::UnexpectedResponse(format!(
                            "expected the JavaScript transformer function `{}` argument to be an int, but found: `{}`",
                            name.as_str(), arg.as_str()
                        ).into()))?
                )
            )),
            (name, arg) => Err(Error::UnexpectedResponse(format!(
                "expected a Javascript transformer function and an argument, got: `{:?}` and `{:?}`",
                name, arg
            ).into()))
        }
    }

    #[inline]
    fn invalid_utf8_err(&self, signature: &[u8]) -> String {
        let error = format!(
            "`decrypt_signature` produced invalid utf-8!\
            Please open an issue on GitHub and paste the whole error message in.\n\
            final signature: {:?}\n\
            transform_plan: {:?}\n\
            transform_map: {:?}",
            signature, self.transform_plan, self.transform_map_dbg()
        );
        log::error!("{}", error);
        eprintln!("{}", error);
        error
    }

    #[inline]
    fn transform_map_dbg(&self) -> String {
        self.transform_map
            .iter()
            .map(|(key, (_f, name))| (key, name))
            .fold(
                String::new(),
                |mut string, (key, name)| {
                    string.push_str(key);
                    string.push_str(" => ");
                    string.push_str(name);
                    string.push(';');
                    string
                },
            )
    }
}

fn get_transform_plan(js: &str) -> Result<Vec<String>> {
    let name = regex::escape(get_initial_function_name(js)?);
    let pattern = Regex::new(&format!(r#"{}=function\(\w\)\{{[a-z=.(")]*;(.*);(?:.+)}}"#, name)).unwrap();
    Ok(
        pattern
            .captures(js)
            .ok_or_else(|| Error::UnexpectedResponse(format!(
                "could not extract the initial JavaScript function: {}",
                pattern
            ).into()))?
            .get(1)
            .expect("the pattern must contain at least one capture group")
            .as_str()
            .split(';')
            .map(str::to_owned)
            .collect()
    )
}

fn get_initial_function_name(js: &str) -> Result<&str> {
    static FUNCTION_PATTERNS: SyncLazy<[Regex; 12]> = SyncLazy::new(|| [
        Regex::new(r"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"\b[a-zA-Z0-9]+\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#).unwrap(),
        Regex::new(r#"(?P<sig>[a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#).unwrap(),
        Regex::new(r#"["']signature["']\s*,\s*(?P<sig>[a-zA-Z0-9$]+)\("#).unwrap(),
        Regex::new(r"\.sig\|\|(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"yt\.akamaized\.net/\)\s*\|\|\s*.*?\s*[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*(?:encodeURIComponent\s*\()?\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"\b[a-zA-Z0-9]+\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"\bc\s*&&\s*a\.set\([^,]+\s*,\s*\([^)]*\)\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"\bc\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*\([^)]*\)\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
        Regex::new(r"\bc\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*\([^)]*\)\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(").unwrap(),
    ]);

    FUNCTION_PATTERNS
        .iter()
        .find_map(|pattern| pattern.captures(js))
        .map(|c| c.get(1).unwrap().as_str())
        .ok_or_else(|| Error::UnexpectedResponse(format!(
            "could not find the JavaScript function name: `{}`",
            js
        ).into()))
}

fn get_transform_map(js: &str, var: &str) -> Result<HashMap<String, TransformerFn>> {
    let transform_object = get_transform_object(js, var)?;
    let mut mapper = HashMap::new();

    for obj in transform_object.split(", ") {
        // AJ:function(a){a.reverse()} => AJ, function(a){a.reverse()}
        let (name, function) = obj
            .split_once(':')
            .ok_or_else(|| Error::UnexpectedResponse(format!(
                "expected the transform-object to contain at least one ':', got {}",
                obj
            ).into()))?;
        let fun = map_functions(function)?;
        mapper.insert(name.to_owned(), fun);
    }

    Ok(mapper)
}

fn map_functions(js_func: &str) -> Result<TransformerFn> {
    static MAPPER: SyncLazy<[(Regex, TransformerFn); 4]> = SyncLazy::new(|| [
        // function(a){a.reverse()}
        (Regex::new(r"\{\w\.reverse\(\)}").unwrap(), (reverse, "reverse")),
        // function(a,b){a.splice(0,b)}
        (Regex::new(r"\{\w\.splice\(0,\w\)}").unwrap(), (splice, "splice")),
        // function(a,b){var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c}
        (Regex::new(r"\{var\s\w=\w\[0];\w\[0]=\w\[\w%\w.length];\w\[\w%\w.length]=\w}").unwrap(), (swap, "swap")),
        // function(a,b){var c=a[0];a[0]=a[b%a.length];a[b]=c}
        (Regex::new(r"\{var\s\w=\w\[0];\w\[0]=\w\[\w%\w.length];\w\[\w]=\w}").unwrap(), (swap, "swap")),
    ]);

    fn reverse(vec: &mut Vec<u8>, _: Option<isize>) {
        vec.reverse();
    }
    fn splice(vec: &mut Vec<u8>, position: Option<isize>) {
        match position {
            None => vec.clear(),
            Some(p) if p.is_positive() && p as usize >= vec.len() => vec.clear(),
            Some(p) if p.is_negative() && -p as usize >= vec.len() => {}
            Some(p) if p.is_negative() => { vec.drain(..vec.len() - p.abs() as usize); }
            Some(p) => { vec.drain(..p as usize); }
        }
    }
    fn swap(vec: &mut Vec<u8>, position: Option<isize>) {
        match position {
            None if vec.is_empty() => vec.push(0),
            None => vec[0] = 0,
            Some(0) => {}
            Some(p) if p.is_positive() && p as usize >= vec.len() => {
                let v0 = vec[0];
                let r = p.abs() as usize % vec.len();
                vec.resize(p as usize, 0);
                vec[0] = vec[r];
                vec.push(v0);
            }
            Some(p) if p.is_negative() && p.abs() as usize % vec.len() == 0 => {}
            Some(p) if p.is_negative() && vec.is_empty() => vec.push(0),
            Some(p) if p.is_negative() => vec[0] = 0,
            Some(p) => {
                let v0 = vec[0];
                vec[0] = vec[p.abs() as usize % vec.len()];
                vec[p.abs() as usize] = v0;
            }
        }
    }

    MAPPER
        .iter()
        .find(|(pattern, _fun)| pattern.is_match(js_func))
        .map(|(_pattern, fun)| *fun)
        .ok_or_else(|| Error::UnexpectedResponse(format!(
            "could not map the JavaScript function `{}` to any Rust equivalent",
            js_func
        ).into()))
}

fn get_transform_object(js: &str, var: &str) -> Result<String> {
    Ok(
        Regex::new(&format!(r"var {}=\{{((?s).*?)}};", regex::escape(var)))
            .unwrap()
            .captures(js)
            .ok_or_else(|| Error::UnexpectedResponse(format!(
                "could not extract the transform-object `{}`",
                var
            ).into()))?
            .get(1)
            .expect("the regex pattern must contain at least one capture group")
            .as_str()
            .replace('\n', " ")
    )
}
