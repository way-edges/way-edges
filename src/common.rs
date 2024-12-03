use std::fmt::Debug;
use std::fmt::Display;

pub fn binary_search_within_range<T: Debug + PartialOrd + Copy + Display>(
    l: &[[T; 2]],
    v: T,
) -> isize {
    if l.is_empty() {
        return -1;
    }
    if l.len() == 1 {
        if v >= l[0][0] && v < l[0][1] {
            return 0;
        } else {
            return -1;
        }
    }

    let mut index = l.len() - 1;
    let mut half = l.len();

    fn half_index(index: &mut usize, half: &mut usize, is_left: bool) {
        *half = (*half / 2).max(1);

        if is_left {
            *index -= *half
        } else {
            *index += *half
        }
    }

    half_index(&mut index, &mut half, true);

    loop {
        let current = l[index];

        if v < current[0] {
            if index == 0 || l[index - 1][1] <= v {
                return -1;
            } else {
                half_index(&mut index, &mut half, true);
            }
        } else if v >= current[1] {
            if index == l.len() - 1 || v < l[index + 1][0] {
                return -1;
            } else {
                half_index(&mut index, &mut half, false);
            }
        } else {
            return index as isize;
        }
    }
}
