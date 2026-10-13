//! Small dense real matrix type with the operations the calculator needs.

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    pub r: usize,
    pub c: usize,
    pub d: Vec<f64>,
}

const EPS: f64 = 1e-12;

impl Matrix {
    pub fn zeros(r: usize, c: usize) -> Self {
        Matrix { r, c, d: vec![0.0; r * c] }
    }

    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.set(i, i, 1.0);
        }
        m
    }

    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Self, String> {
        let r = rows.len();
        if r == 0 || rows[0].is_empty() {
            return Err("Empty matrix".into());
        }
        let c = rows[0].len();
        if rows.iter().any(|row| row.len() != c) {
            return Err("Matrix rows must have equal length".into());
        }
        Ok(Matrix { r, c, d: rows.into_iter().flatten().collect() })
    }

    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.d[i * self.c + j]
    }

    #[inline]
    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.d[i * self.c + j] = v;
    }

    pub fn is_square(&self) -> bool {
        self.r == self.c
    }

    pub fn add(&self, o: &Matrix) -> Result<Matrix, String> {
        self.zip(o, |a, b| a + b)
    }

    pub fn sub(&self, o: &Matrix) -> Result<Matrix, String> {
        self.zip(o, |a, b| a - b)
    }

    fn zip(&self, o: &Matrix, f: impl Fn(f64, f64) -> f64) -> Result<Matrix, String> {
        if self.r != o.r || self.c != o.c {
            return Err("Matrix dimensions do not match".into());
        }
        Ok(Matrix {
            r: self.r,
            c: self.c,
            d: self.d.iter().zip(&o.d).map(|(a, b)| f(*a, *b)).collect(),
        })
    }

    pub fn scale(&self, s: f64) -> Matrix {
        Matrix { r: self.r, c: self.c, d: self.d.iter().map(|a| a * s).collect() }
    }

    pub fn mul(&self, o: &Matrix) -> Result<Matrix, String> {
        if self.c != o.r {
            return Err("Matrix dimensions do not match for ×".into());
        }
        let mut m = Matrix::zeros(self.r, o.c);
        for i in 0..self.r {
            for j in 0..o.c {
                let mut s = 0.0;
                for k in 0..self.c {
                    s += self.get(i, k) * o.get(k, j);
                }
                m.set(i, j, s);
            }
        }
        Ok(m)
    }

    pub fn transpose(&self) -> Matrix {
        let mut m = Matrix::zeros(self.c, self.r);
        for i in 0..self.r {
            for j in 0..self.c {
                m.set(j, i, self.get(i, j));
            }
        }
        m
    }

    pub fn trace(&self) -> Result<f64, String> {
        if !self.is_square() {
            return Err("Trace needs a square matrix".into());
        }
        Ok((0..self.r).map(|i| self.get(i, i)).sum())
    }

    pub fn det(&self) -> Result<f64, String> {
        if !self.is_square() {
            return Err("Determinant needs a square matrix".into());
        }
        let n = self.r;
        let mut a = self.clone();
        let mut det = 1.0;
        for col in 0..n {
            let mut p = col;
            for i in col + 1..n {
                if a.get(i, col).abs() > a.get(p, col).abs() {
                    p = i;
                }
            }
            if a.get(p, col).abs() < EPS {
                return Ok(0.0);
            }
            if p != col {
                a.swap_rows(p, col);
                det = -det;
            }
            let piv = a.get(col, col);
            det *= piv;
            for i in col + 1..n {
                let f = a.get(i, col) / piv;
                if f != 0.0 {
                    for j in col..n {
                        let v = a.get(i, j) - f * a.get(col, j);
                        a.set(i, j, v);
                    }
                }
            }
        }
        Ok(det)
    }

    fn swap_rows(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        for j in 0..self.c {
            self.d.swap(a * self.c + j, b * self.c + j);
        }
    }

    pub fn inverse(&self) -> Result<Matrix, String> {
        if !self.is_square() {
            return Err("Inverse needs a square matrix".into());
        }
        let n = self.r;
        let mut a = Matrix::zeros(n, 2 * n);
        for i in 0..n {
            for j in 0..n {
                a.set(i, j, self.get(i, j));
            }
            a.set(i, n + i, 1.0);
        }
        let scale = self.d.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-300);
        for col in 0..n {
            let mut p = col;
            for i in col + 1..n {
                if a.get(i, col).abs() > a.get(p, col).abs() {
                    p = i;
                }
            }
            if a.get(p, col).abs() < EPS * scale {
                return Err("Matrix is singular".into());
            }
            a.swap_rows(p, col);
            let piv = a.get(col, col);
            for j in 0..2 * n {
                let v = a.get(col, j) / piv;
                a.set(col, j, v);
            }
            for i in 0..n {
                if i != col {
                    let f = a.get(i, col);
                    if f != 0.0 {
                        for j in 0..2 * n {
                            let v = a.get(i, j) - f * a.get(col, j);
                            a.set(i, j, v);
                        }
                    }
                }
            }
        }
        let mut m = Matrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                m.set(i, j, a.get(i, n + j));
            }
        }
        Ok(m)
    }

    /// Reduced row echelon form, plus the rank.
    pub fn rref(&self) -> (Matrix, usize) {
        let mut a = self.clone();
        let scale = self.d.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-300);
        let mut row = 0;
        for col in 0..a.c {
            if row >= a.r {
                break;
            }
            let mut p = row;
            for i in row + 1..a.r {
                if a.get(i, col).abs() > a.get(p, col).abs() {
                    p = i;
                }
            }
            if a.get(p, col).abs() < 1e-10 * scale {
                for i in row..a.r {
                    a.set(i, col, 0.0);
                }
                continue;
            }
            a.swap_rows(p, row);
            let piv = a.get(row, col);
            for j in 0..a.c {
                let v = a.get(row, j) / piv;
                a.set(row, j, v);
            }
            for i in 0..a.r {
                if i != row {
                    let f = a.get(i, col);
                    if f != 0.0 {
                        for j in 0..a.c {
                            let v = a.get(i, j) - f * a.get(row, j);
                            a.set(i, j, v);
                        }
                    }
                }
            }
            row += 1;
        }
        for v in a.d.iter_mut() {
            if v.abs() < 1e-12 {
                *v = 0.0;
            }
        }
        (a, row)
    }

    pub fn pow(&self, k: i64) -> Result<Matrix, String> {
        if !self.is_square() {
            return Err("Power needs a square matrix".into());
        }
        if k.abs() > 10_000 {
            return Err("Exponent too large".into());
        }
        let mut base = if k < 0 { self.inverse()? } else { self.clone() };
        let mut e = k.unsigned_abs();
        let mut acc = Matrix::identity(self.r);
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(&base)?;
            }
            base = base.mul(&base)?;
            e >>= 1;
        }
        Ok(acc)
    }

    /// Coefficients of the characteristic polynomial det(λI − A), highest power first
    /// (Faddeev–LeVerrier). Fine for the small matrices the calculator works with.
    pub fn char_poly(&self) -> Result<Vec<f64>, String> {
        if !self.is_square() {
            return Err("Eigenvalues need a square matrix".into());
        }
        let n = self.r;
        let mut coef = vec![1.0];
        let mut m = Matrix::zeros(n, n);
        for k in 1..=n {
            // M_k = A·M_{k−1} + c_{n−k+1}·I,  c_{n−k} = −tr(A·M_k)/k
            let am = self.mul(&m).unwrap();
            m = am.add(&Matrix::identity(n).scale(*coef.last().unwrap())).unwrap();
            let c = -self.mul(&m).unwrap().trace().unwrap() / k as f64;
            coef.push(c);
        }
        Ok(coef)
    }

    /// Eigenvalues as complex numbers, sorted by real part then imaginary part.
    pub fn eigenvalues(&self) -> Result<Vec<num_complex::Complex64>, String> {
        let coef = self.char_poly()?;
        let mut v = crate::numeric::poly_roots(&coef)?;
        // Tidy tiny imaginary parts left by the root finder.
        let scale = coef.iter().fold(1.0f64, |m, c| m.max(c.abs()));
        for z in v.iter_mut() {
            if z.im.abs() < 1e-9 * scale.max(1.0) {
                z.im = 0.0;
            }
        }
        v.sort_by(|a, b| (a.re, a.im).partial_cmp(&(b.re, b.im)).unwrap_or(std::cmp::Ordering::Equal));
        Ok(v)
    }

    /// Eigenvector for a real eigenvalue by inverse iteration; unit length, first non-zero
    /// component positive.
    pub fn eigenvector(&self, lambda: f64) -> Result<Vec<f64>, String> {
        let n = self.r;
        // (A − λI) shifted slightly off the eigenvalue so it stays invertible.
        let shift = lambda + 1e-7 * (1.0 + lambda.abs());
        let shifted = self.sub(&Matrix::identity(n).scale(shift))?;
        let inv = match shifted.inverse() {
            Ok(m) => m,
            Err(_) => self.sub(&Matrix::identity(n).scale(shift * (1.0 + 1e-6) + 1e-6))?.inverse()?,
        };
        let mut v = Matrix::from_rows((0..n).map(|i| vec![1.0 / (i as f64 + 1.5)]).collect())?;
        for _ in 0..40 {
            v = inv.mul(&v)?;
            let norm = v.norm();
            if norm == 0.0 || !norm.is_finite() {
                return Err("Could not find an eigenvector".into());
            }
            v = v.scale(1.0 / norm);
        }
        let mut out: Vec<f64> = v.d.clone();
        if let Some(first) = out.iter().find(|x| x.abs() > 1e-9) {
            if *first < 0.0 {
                out.iter_mut().for_each(|x| *x = -*x);
            }
        }
        out.iter_mut().for_each(|x| {
            if x.abs() < 1e-10 {
                *x = 0.0;
            }
        });
        Ok(out)
    }

    pub fn norm(&self) -> f64 {
        self.d.iter().map(|v| v * v).sum::<f64>().sqrt()
    }
}
