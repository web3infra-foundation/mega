use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::Arc,
};

pub trait Storable: Sized {
    type Location: ?Sized;
    type Strategy: StorageStrategy<Self>;
    fn load(location: &Self::Location) -> Result<Self, io::Error> {
        Self::Strategy::load(location)
    }
    fn store(&self, location: &Self::Location) -> Result<(), io::Error> {
        Self::Strategy::store(self, location)
    }
}

pub trait StorageStrategy<T: Storable> {
    fn load(path: &T::Location) -> Result<T, io::Error>;
    fn store(obj: &T, location: &T::Location) -> Result<(), io::Error>;
}

impl<T: Storable> Storable for Arc<T> {
    type Location = T::Location;
    type Strategy = ArcIndirectStorageStrategy<Arc<T>>;
}

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
        let path = location.with_extension("temp");
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path.clone())?;
            file.write_all(&data)?;
        }
        let final_path = path.with_extension("");
        fs::rename(&path, final_path.clone())?;
        Ok(())
    }
}
