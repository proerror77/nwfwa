pub fn take_flag_value(args: &mut Vec<String>, flag: &str) -> anyhow::Result<String> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        anyhow::bail!("missing required flag {flag}");
    };
    args.remove(index);
    if index >= args.len() {
        anyhow::bail!("missing value for flag {flag}");
    }
    Ok(args.remove(index))
}

pub fn take_optional_flag_value(
    args: &mut Vec<String>,
    flag: &str,
) -> anyhow::Result<Option<String>> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    args.remove(index);
    if index >= args.len() {
        anyhow::bail!("missing value for flag {flag}");
    }
    Ok(Some(args.remove(index)))
}

pub fn take_optional_f64_flag(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Option<f64>> {
    take_optional_flag_value(args, flag)?
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|error| anyhow::anyhow!("invalid {flag}: {error}"))
        })
        .transpose()
}

pub fn take_optional_u64_flag(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Option<u64>> {
    take_optional_flag_value(args, flag)?
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| anyhow::anyhow!("invalid {flag}: {error}"))
        })
        .transpose()
}

pub fn take_optional_usize_flag(
    args: &mut Vec<String>,
    flag: &str,
) -> anyhow::Result<Option<usize>> {
    take_optional_flag_value(args, flag)?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| anyhow::anyhow!("invalid {flag}: {error}"))
        })
        .transpose()
}

pub fn take_repeated_flag_value(args: &mut Vec<String>, flag: &str) -> anyhow::Result<Vec<String>> {
    let mut values = Vec::new();
    while let Some(index) = args.iter().position(|arg| arg == flag) {
        args.remove(index);
        if index >= args.len() {
            anyhow::bail!("missing value for flag {flag}");
        }
        values.push(args.remove(index));
    }
    if values.is_empty() {
        anyhow::bail!("missing required flag {flag}");
    }
    Ok(values)
}

pub fn take_bool_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return false;
    };
    args.remove(index);
    true
}
