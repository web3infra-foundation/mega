//! # Storable
//!
//! This module provides traits and implementations for storing and loading
//! objects using various storage strategies. The primary traits are
//! `Storable` and `StorageStrategy`, which allow defining how objects are
//! serialized and deserialized from their storage locations.
//!
//! ## Overview
//!
//! - **Storable:** Defines the interface for objects that can be stored and loaded.
//! - **StorageStrategy:** Specifies the strategy used for storage operations.
//!
//! ## Examples
//!
//! ### Using `DefaultFileStorageStrategy`
//! 
//! If you have a struct that implements `Serialize` and `Deserialize`, you can use
//! [`Storable`] for free by specifying the `Location` type as `Path` and the `Strategy`
//! as `DefaultFileStorageStrategy`:
//! 
//! ```rust
//! use std::path::Path;
//! use serde::{Serialize, Deserialize};
//! 
//! #[derive(Serialize, Deserialize)]
//! struct MyData {
//!    value: i32,
//! }
//! 
//! impl Storable for MyData {
//!    type Location = Path;
//!    type Strategy = DefaultFileStorageStrategy;
//! }
//! ```
//!
//! See the example under [`DefaultFileStorageStrategy`].
//!
//! ### Implementing a custom storage strategy
//!
//! See the example under [`StorageStrategy`].

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::Arc,
};

/// Trait representing an object that can be stored and loaded.
///
/// This trait defines the necessary methods and associated types
/// required for an object to be storable using a specified strategy.
pub trait Storable: Sized {
    /// The type representing the location where the object is stored.
    type Location: ?Sized;
    /// The strategy used for storage operations.
    type Strategy: StorageStrategy<Self>;

    /// Loads an instance from the specified location.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use mercury::utils::storable::{Storable, DefaultFileStorageStrategy};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct MyData {
    ///     value: i32,
    /// }
    ///
    /// impl Storable for MyData {
    ///     type Location = Path;
    ///     type Strategy = DefaultFileStorageStrategy;
    /// }
    ///
    /// fn load_data() -> Result<MyData, std::io::Error> {
    ///     MyData::load(&Path::new("data.bin"))
    /// }
    /// ```
    fn load(location: &Self::Location) -> Result<Self, io::Error> {
        Self::Strategy::load(location)
    }

    /// Stores the current instance to the specified location.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use mercury::utils::storable::{Storable, DefaultFileStorageStrategy};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct MyData {
    ///     value: i32,
    /// }
    ///
    /// impl Storable for MyData {
    ///     type Location = Path;
    ///     type Strategy = DefaultFileStorageStrategy;
    /// }
    ///
    /// fn store_data(data: &MyData) -> Result<(), std::io::Error> {
    ///     data.store(&Path::new("data.bin"))
    /// }
    /// ```
    fn store(&self, location: &Self::Location) -> Result<(), io::Error> {
        Self::Strategy::store(self, location)
    }
}

/// Trait defining the strategy for storing and loading `Storable` objects.
///
/// Implement this trait to define custom storage mechanisms.
///
/// This trait is not intended to be called directly, call the `load` and `store` methods
/// on the `Storable` trait instead.
///
/// # Implementing a custom storage strategy
///
/// ```rust
/// use std::cell::RefCell;
/// use mercury::utils::storable::{Storable, StorageStrategy};
///
/// type CustomLocation = RefCell<i32>;
/// struct CustomStorage;
///
/// impl StorageStrategy<MyData> for CustomStorage {
///     fn load(location: &CustomLocation) -> Result<MyData, std::io::Error> {
///         // Custom load logic
///         Ok(MyData { value: *location.borrow() })
///     }
///
///     fn store(obj: &MyData, location: &CustomLocation) -> Result<(), std::io::Error> {
///         // Custom store logic
///         *location.borrow_mut() = obj.value;
///         Ok(())
///     }
/// }
///
/// struct MyData {
///     value: i32,
/// }
///
/// impl Storable for MyData {
///     type Location = CustomLocation;
///     type Strategy = CustomStorage;
/// }
///
/// fn main() -> Result<(), std::io::Error> {
///     let location = RefCell::new(0);
///     let data = MyData { value: 42 };
///     data.store(&location)?;
///     let loaded = MyData::load(&location)?;
///     assert_eq!(loaded.value, 42);
///     Ok(())
/// }
/// ```
pub trait StorageStrategy<T: Storable> {
    /// Loads an object of type `T` from the specified location.
    fn load(location: &T::Location) -> Result<T, io::Error>;

    /// Stores the given object of type `T` to the specified location.
    fn store(obj: &T, location: &T::Location) -> Result<(), io::Error>;
}

/// Implement `Storable` for `Arc<T>` where `T` is `Storable`.
impl<T: Storable> Storable for Arc<T> {
    type Location = T::Location;
    type Strategy = ArcIndirectStorageStrategy<Arc<T>>;
}

/// Indirect storage strategy for `Arc<T>`.
///
/// This is the storage strategy for `Arc<T>` objects, which delegates
/// storage operations to the underlying object `T`.
///
/// This implementation allows storing and loading objects wrapped in an `Arc`.
///
/// # Examples
///
/// ```rust
/// use std::sync::Arc;
/// use std::path::Path;
/// use serde::{Serialize, Deserialize};
/// use mercury::utils::storable::{Storable, StorageStrategy, DefaultFileStorageStrategy};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyData {
///     value: i32,
/// }
///
/// impl Storable for MyData {
///     type Location = Path;
///     type Strategy = DefaultFileStorageStrategy;
/// }
///
/// fn main() -> Result<(), std::io::Error> {
///     let data = Arc::new(MyData { value: 42 });
///     data.store(&Path::new("data.bin"))?;
///     let loaded = MyData::load(&Path::new("data.bin"))?;
///     assert_eq!(loaded.value, 42);
///     std::fs::remove_file("data.bin")?;
///     Ok(())
/// }
/// ```
pub struct ArcIndirectStorageStrategy<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> StorageStrategy<Arc<T>> for ArcIndirectStorageStrategy<Arc<T>>
where
    T: Storable,
{
    fn load(location: &T::Location) -> Result<Arc<T>, io::Error> {
        Ok(Arc::new(T::load(location)?))
    }

    fn store(obj: &Arc<T>, location: &T::Location) -> Result<(), io::Error> {
        obj.as_ref().store(location)
    }
}

/// Default file storage strategy using serialization.
///
/// This strategy serializes objects to binary format using `bincode`.
/// This `StorageStrategy` is automatically implemented for any type that
/// implements [`Serialize`] and [`Deserialize`] traits.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use serde::{Serialize, Deserialize};
/// use mercury::utils::storable::{Storable, DefaultFileStorageStrategy};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyData {
///     value: i32,
/// }
///
/// impl Storable for MyData {
///     type Location = Path;
///     type Strategy = DefaultFileStorageStrategy;
/// }
///
/// fn main() -> Result<(), std::io::Error> {
///     let data = MyData { value: 42 };
///     data.store(&Path::new("data.bin"))?;
///     let loaded = MyData::load(&Path::new("data.bin"))?;
///     assert_eq!(loaded.value, 42);
///     std::fs::remove_file("data.bin")?;
///     Ok(())
/// }
/// ```
pub struct DefaultFileStorageStrategy;

impl<T> StorageStrategy<T> for DefaultFileStorageStrategy
where
    T: Serialize + for<'de> Deserialize<'de> + Storable<Location = Path>,
{
    fn load(location: &T::Location) -> Result<T, io::Error> {
        let data = fs::read(location)?;
        let obj: T =
            bincode::deserialize(&data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(obj)
    }

    fn store(obj: &T, location: &T::Location) -> Result<(), io::Error> {
        if location.exists() {
            return Ok(());
        }
        let data = bincode::serialize(obj).unwrap();
        let tmp_path = location.with_extension("temp");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(&data)?;
        fs::rename(tmp_path, location)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::MERCURY_DEFAULT_TMP_DIR;

    use super::*;
    use std::path::{Path, PathBuf};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MyData {
        value: i32,
        complex_field: (Vec<i32>, String),
    }

    impl Storable for MyData {
        type Location = Path;
        type Strategy = DefaultFileStorageStrategy;
    }

    #[test]
    fn test_default_file_storage_strategy() {
        fs::create_dir_all(MERCURY_DEFAULT_TMP_DIR).unwrap();
        let data = MyData { value: 42, complex_field: (vec![1, 2, 3], "hello".to_string()) };
        let path = PathBuf::from(MERCURY_DEFAULT_TMP_DIR).join("data.bin");
        data.store(&path).unwrap();
        let loaded = MyData::load(&path).unwrap();
        assert_eq!(loaded, data);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_arc_indirect_storage_strategy() {
        fs::create_dir_all(MERCURY_DEFAULT_TMP_DIR).unwrap();
        let data = Arc::new(MyData { value: 42 , complex_field: (vec![1, 2, 3], "hello".to_string()) });
        let path = PathBuf::from(MERCURY_DEFAULT_TMP_DIR).join("arcdata.bin");
        data.store(&path).unwrap();
        let loaded = MyData::load(&path).unwrap();
        assert_eq!(*data, loaded);
        fs::remove_file(path).unwrap();
    }
}