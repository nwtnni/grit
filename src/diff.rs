use std::ops;

pub fn myers<A, B>(a: &[A], b: &[B]) -> usize
where
    A: PartialEq<B>,
{
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = n + m;

    let mut v = Ring(vec![0; 2 * max as usize + 1]);

    for d in 0..max {
        for k in (-d..=d).step_by(2) {
            let mut x = if k == -d || (k != d && v[k - 1] < v[k + 1]) {
                v[k + 1]
            } else {
                v[k - 1] + 1
            };

            let mut y = x - k;

            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }

            v[k] = x;

            if x >= n && y >= m {
                return d as usize;
            }
        }
    }

    unreachable!()
}

#[derive(Clone, Debug)]
struct Ring<T>(Vec<T>);

impl<T> ops::Index<isize> for Ring<T> {
    type Output = T;
    fn index(&self, index: isize) -> &Self::Output {
        let mid = self.0.len() >> 1;
        &self.0[(mid as isize + index) as usize]
    }
}

impl<T> ops::IndexMut<isize> for Ring<T> {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        let mid = self.0.len() >> 1;
        &mut self.0[(mid as isize + index) as usize]
    }
}

#[test]
fn smoke() {
    assert_eq!(myers(b"ABCABBA", b"CBABAC"), 5);
}
