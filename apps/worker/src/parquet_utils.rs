use anyhow::{anyhow, bail, Context};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn resolve_parquet_files(
    base_dir: &Path,
    data_uri: &str,
) -> anyhow::Result<Vec<PathBuf>> {
    let path = PathBuf::from(data_uri);
    let path = if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    };

    if path.is_file() {
        ensure_parquet_path(&path)?;
        return Ok(vec![path]);
    }

    if path.is_dir() {
        let mut files = fs::read_dir(&path)
            .with_context(|| format!("read parquet directory {}", path.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "parquet")
            })
            .collect::<Vec<_>>();
        files.sort();
        return Ok(files);
    }

    Err(anyhow!(
        "parquet data_uri does not exist: {}",
        path.display()
    ))
}

pub(crate) fn ensure_parquet_path(path: &Path) -> anyhow::Result<()> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("parquet") => Ok(()),
        _ => bail!("data_uri file must end with .parquet: {}", path.display()),
    }
}

pub(crate) fn column_value_at(array: &dyn arrow_array::Array, index: usize) -> Option<String> {
    use arrow_array::{
        BooleanArray, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };

    if array.is_null(index) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return Some(values.value(index).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(index).to_string());
    }
    None
}

pub(crate) fn column_values(array: &dyn arrow_array::Array) -> Vec<String> {
    use arrow_array::{
        Array, BooleanArray, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
        LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };

    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return (0..values.len())
            .filter(|index| !values.is_null(*index))
            .map(|index| values.value(index).to_string())
            .collect();
    }
    if let Some(values) = array.as_any().downcast_ref::<Int8Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int16Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt8Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt16Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt32Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<UInt64Array>() {
        return primitive_values(values, |value| value.to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return primitive_values(values, |value| value.to_string());
    }

    Vec::new()
}

fn primitive_values<T, F>(array: &arrow_array::PrimitiveArray<T>, format: F) -> Vec<String>
where
    T: arrow_array::ArrowPrimitiveType,
    F: Fn(T::Native) -> String,
{
    use arrow_array::Array;

    (0..array.len())
        .filter(|index| !array.is_null(*index))
        .map(|index| format(array.value(index)))
        .collect()
}
