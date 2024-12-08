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

pub fn binary_search_end<T: Debug + PartialOrd + Copy + Display + Default>(l: &[T], v: T) -> isize {
    if l.is_empty() {
        return -1;
    }
    if l.len() == 1 {
        if v >= T::default() && v < l[0] {
            return 0;
        } else {
            return -1;
        }
    }

    let mut index = 0;
    let max_index = l.len() - 1;
    let mut get_half = {
        let mut half = l.len();
        move || {
            half = (half / 2).max(1);
            half
        }
    };

    loop {
        let current = l[index];

        if v < current {
            // if at the first, or there's no smaller to the left
            if index == 0 || v >= l[index - 1] {
                return index as isize;
            }
            index -= get_half();
        } else {
            // if it's the last
            if index == max_index {
                return -1;
            }

            // if smaller than the right
            if v < l[index + 1] {
                return (index + 1) as isize;
            }
            index += get_half();
        }
    }
}

// pub trait VecInto<D> {
//     fn vec_into(self) -> Vec<D>;
// }
//
// impl<E, D> VecInto<D> for Vec<E>
// where
//     D: From<E>,
// {
//     fn vec_into(self) -> Vec<D> {
//         self.into_iter().map(std::convert::Into::into).collect()
//     }
// }
