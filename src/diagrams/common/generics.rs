fn tildes(text: &str) -> usize {
    text.matches('~').count()
}

fn pair_tildes(text: &str) -> String {
    let count = tildes(text);
    if count <= 1 {
        return text.to_owned();
    }
    let lead = count % 2 != 0 && text.starts_with('~');
    let mut chars: Vec<char> = text.chars().collect();
    let start = usize::from(lead);
    let at: Vec<usize> = (start..chars.len()).filter(|&i| chars[i] == '~').collect();
    let (mut lo, mut hi) = (0, at.len() as isize - 1);
    while (lo as isize) < hi {
        chars[at[lo]] = '<';
        chars[at[hi as usize]] = '>';
        lo += 1;
        hi -= 1;
    }
    chars.into_iter().collect()
}

pub fn parse_generic_types(input: &str) -> String {
    if !input.contains('~') {
        return input.to_owned();
    }
    let mut sets: Vec<String> = Vec::new();
    for (i, part) in input.split(',').enumerate() {
        if i > 0 {
            sets.push(",".to_owned());
        }
        sets.push(part.to_owned());
    }
    let mut output: Vec<String> = Vec::new();
    let mut i = 0;
    while i < sets.len() {
        let mut set = sets[i].clone();
        if set == ","
            && i > 0
            && i + 1 < sets.len()
            && tildes(&sets[i - 1]) == 1
            && tildes(&sets[i + 1]) == 1
        {
            set = format!("{},{}", sets[i - 1], sets[i + 1]);
            i += 1;
            output.pop();
        }
        output.push(pair_tildes(&set));
        i += 1;
    }
    output.concat()
}
