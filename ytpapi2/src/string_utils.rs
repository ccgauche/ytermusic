use std::str::FromStr;

pub trait StringUtils {
    fn after(&self, needle: &str) -> Option<&str>;
    fn before(&self, needle: &str) -> Option<&str>;
    fn between(&self, start: &str, end: &str) -> Option<&str>;

    fn to_owned_(&self) -> Option<String>;
    fn parse_<T: FromStr>(&self) -> Option<T>;
    fn trim_(&self) -> Option<&str>;
}

impl StringUtils for &str {
    fn after(&self, needle: &str) -> Option<&str> {
        Some(&self[self.find(needle)? + needle.len()..])
    }

    fn before(&self, needle: &str) -> Option<&str> {
        Some(&self[..self.find(needle)?])
    }

    fn between(&self, start: &str, end: &str) -> Option<&str> {
        let string: &str = &self[self.find(start)? + start.len()..];
        Some(&string[..string.find(end)?])
    }

    fn to_owned_(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn parse_<T: FromStr>(&self) -> Option<T> {
        T::from_str(self).ok()
    }

    fn trim_(&self) -> Option<&str> {
        Some(str::trim(self))
    }
}

impl StringUtils for String {
    fn after(&self, needle: &str) -> Option<&str> {
        Some(&self[self.find(needle)? + needle.len()..])
    }

    fn before(&self, needle: &str) -> Option<&str> {
        Some(&self[..self.find(needle)?])
    }

    fn between(&self, start: &str, end: &str) -> Option<&str> {
        let string: &str = &self[self.find(start)? + start.len()..];
        Some(&string[..string.find(end)?])
    }

    fn to_owned_(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn parse_<T: FromStr>(&self) -> Option<T> {
        T::from_str(self).ok()
    }

    fn trim_(&self) -> Option<&str> {
        Some(str::trim(self))
    }
}

impl<T: StringUtils> StringUtils for Option<T> {
    fn after(&self, needle: &str) -> Option<&str> {
        self.as_ref().and_then(|string| string.after(needle))
    }

    fn before(&self, needle: &str) -> Option<&str> {
        self.as_ref().and_then(|string| string.before(needle))
    }

    fn between(&self, start: &str, end: &str) -> Option<&str> {
        self.as_ref().and_then(|string| string.between(start, end))
    }

    fn to_owned_(&self) -> Option<String> {
        self.as_ref().and_then(|string| string.to_owned_())
    }

    fn parse_<E: FromStr>(&self) -> Option<E> {
        self.as_ref().and_then(|string| string.parse_::<E>())
    }

    fn trim_(&self) -> Option<&str> {
        self.as_ref().and_then(|string| string.trim_())
    }
}
