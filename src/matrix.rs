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

    /// Basis of the null space, one vector per column, read off the reduced row echelon form.
    pub fn nullspace(&self) -> Result<Matrix, String> {
        let (r, rank) = self.rref();
        // Pivot columns: the first non-zero entry of each of the first `rank` rows.
        let mut pivots = Vec::new();
        for i in 0..rank {
            if let Some(j) = (0..r.c).find(|j| r.get(i, *j) != 0.0) {
                pivots.push(j);
            }
        }
        let free: Vec<usize> = (0..r.c).filter(|j| !pivots.contains(j)).collect();
        if free.is_empty() {
            return Err("Only the zero vector: the columns are independent".into());
        }
        let mut rows = vec![vec![0.0; free.len()]; r.c];
        for (k, &fcol) in free.iter().enumerate() {
            rows[fcol][k] = 1.0;
            for (i, &pcol) in pivots.iter().enumerate() {
                rows[pcol][k] = -r.get(i, fcol);
            }
        }
        Matrix::from_rows(rows)
    }

    /// LU decomposition with partial pivoting: returns (P, L, U) with P·A = L·U.
    pub fn lu(&self) -> Result<(Matrix, Matrix, Matrix), String> {
        if !self.is_square() {
            return Err("LU needs a square matrix".into());
        }
        let n = self.r;
        let mut u = self.clone();
        let mut l = Matrix::identity(n);
        let mut perm = Matrix::identity(n);
        for col in 0..n {
            let mut p = col;
            for i in col + 1..n {
                if u.get(i, col).abs() > u.get(p, col).abs() {
                    p = i;
                }
            }
            if u.get(p, col).abs() < EPS {
                continue;
            }
            if p != col {
                u.swap_rows(p, col);
                perm.swap_rows(p, col);
                for j in 0..col {
                    let t = l.get(p, j);
                    l.set(p, j, l.get(col, j));
                    l.set(col, j, t);
                }
            }
            for i in col + 1..n {
                let f = u.get(i, col) / u.get(col, col);
                l.set(i, col, f);
                for j in col..n {
                    let v = u.get(i, j) - f * u.get(col, j);
                    u.set(i, j, v);
                }
            }
        }
        for v in u.d.iter_mut() {
            if v.abs() < 1e-12 {
                *v = 0.0;
            }
        }
        Ok((perm, l, u))
    }

    /// QR decomposition by modified Gram–Schmidt: A = Q·R, Q with orthonormal columns.
    pub fn qr(&self) -> Result<(Matrix, Matrix), String> {
        let (m, n) = (self.r, self.c);
        let mut q = Matrix::zeros(m, n);
        let mut r = Matrix::zeros(n, n);
        let mut cols: Vec<Vec<f64>> = (0..n).map(|j| (0..m).map(|i| self.get(i, j)).collect()).collect();
        for j in 0..n {
            for k in 0..j {
                let qk: Vec<f64> = (0..m).map(|i| q.get(i, k)).collect();
                let dot: f64 = qk.iter().zip(&cols[j]).map(|(a, b)| a * b).sum();
                r.set(k, j, dot);
                for i in 0..m {
                    cols[j][i] -= dot * qk[i];
                }
            }
            let norm = cols[j].iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm < 1e-10 {
                return Err("The columns are not independent".into());
            }
            r.set(j, j, norm);
            for i in 0..m {
                q.set(i, j, cols[j][i] / norm);
            }
        }
        Ok((q, r))
    }

    /// Matrix exponential by scaling and squaring with a Taylor series.
    pub fn expm(&self) -> Result<Matrix, String> {
        if !self.is_square() {
            return Err("expm needs a square matrix".into());
        }
        let n = self.r;
        let norm = self.norm();
        let k = if norm > 0.5 { (norm / 0.5).log2().ceil() as i32 } else { 0 };
        let a = self.scale(0.5f64.powi(k));
        let mut term = Matrix::identity(n);
        let mut sum = Matrix::identity(n);
        for i in 1..=18 {
            term = term.mul(&a)?.scale(1.0 / i as f64);
            sum = sum.add(&term)?;
        }
        for _ in 0..k {
            sum = sum.mul(&sum)?;
        }
        Ok(sum)
    }

    /// 2-norm condition number √(λmax/λmin) of AᵀA; ∞ for a singular matrix.
    pub fn cond(&self) -> Result<f64, String> {
        let ata = self.transpose().mul(self)?;
        let ev = ata.eigenvalues()?;
        let re: Vec<f64> = ev.iter().map(|z| z.re.max(0.0)).collect();
        let max = re.iter().cloned().fold(0.0, f64::max);
        let min = re.iter().cloned().fold(f64::INFINITY, f64::min);
        if min <= 1e-14 * max.max(1.0) {
            return Ok(f64::INFINITY);
        }
        Ok((max / min).sqrt())
    }

    /// Two matrices stacked vertically, for results that come in pairs (L over U, Q over R).
    pub fn stack(top: &Matrix, bottom: &Matrix) -> Result<Matrix, String> {
        if top.c != bottom.c {
            return Err("Cannot stack matrices of different widths".into());
        }
        let mut d = top.d.clone();
        d.extend_from_slice(&bottom.d);
        Ok(Matrix { r: top.r + bottom.r, c: top.c, d })
    }

    pub fn norm(&self) -> f64 {
        self.d.iter().map(|v| v * v).sum::<f64>().sqrt()
    }
}
