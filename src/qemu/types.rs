//! Common types used throughout the ezkvm project
//!
//! This module defines shared types and newtypes for better type safety.

/// A newtype wrapper around `Vec<String>` for QEMU command-line arguments.
///
/// This provides type safety and prevents accidentally mixing QEMU args
/// with other string vectors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QemuArgs(Vec<String>);

impl QemuArgs {
    /// Create a new empty QEMU arguments collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Create QEMU arguments from a vector of strings
    pub fn from_vec(args: Vec<String>) -> Self {
        Self(args)
    }

    /// Add a single argument
    pub fn push(&mut self, arg: String) {
        self.0.push(arg);
    }

    /// Add a single argument from a string slice
    pub fn push_str(&mut self, arg: &str) {
        self.0.push(arg.to_string());
    }

    /// Add multiple arguments
    pub fn extend(&mut self, args: impl IntoIterator<Item = String>) {
        self.0.extend(args);
    }

    /// Get the number of arguments
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if there are no arguments
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clear all arguments
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Get an iterator over the arguments
    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }

    /// Consume the QemuArgs and return the inner Vec<String>
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

impl IntoIterator for QemuArgs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a QemuArgs {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl From<Vec<String>> for QemuArgs {
    fn from(args: Vec<String>) -> Self {
        Self(args)
    }
}

impl From<QemuArgs> for Vec<String> {
    fn from(args: QemuArgs) -> Vec<String> {
        args.0
    }
}

impl AsRef<[String]> for QemuArgs {
    fn as_ref(&self) -> &[String] {
        &self.0
    }
}

impl std::ops::Deref for QemuArgs {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for QemuArgs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qemu_args_new() {
        let args = QemuArgs::new();
        assert!(args.is_empty());
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_qemu_args_push() {
        let mut args = QemuArgs::new();
        args.push_str("-m");
        args.push("1024M".to_string());

        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-m");
        assert_eq!(args[1], "1024M");
    }

    #[test]
    fn test_qemu_args_from_vec() {
        let vec = vec!["-cpu".to_string(), "host".to_string()];
        let args = QemuArgs::from_vec(vec.clone());

        assert_eq!(args.len(), 2);
        assert_eq!(args.into_inner(), vec);
    }

    #[test]
    fn test_qemu_args_extend() {
        let mut args = QemuArgs::new();
        args.push_str("-machine");
        args.extend(vec!["type=q35".to_string(), "accel=kvm".to_string()]);

        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "-machine");
        assert_eq!(args[1], "type=q35");
        assert_eq!(args[2], "accel=kvm");
    }

    #[test]
    fn test_qemu_args_conversion() {
        let original = vec!["-smp".to_string(), "4".to_string()];
        let args = QemuArgs::from(original.clone());
        let back_to_vec: Vec<String> = args.into();

        assert_eq!(back_to_vec, original);
    }
}