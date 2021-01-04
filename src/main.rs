use ff::Field;
use ff::PrimeField;
use rand_core::RngCore;

/// This prime field has the greatest 128-bit prime as modulus. Because of the ff crate, each field
/// element is 192bit (3*8 bytes) instead of 128 (2*8) bytes: take care when sampling random bytes.
#[derive(PrimeField)]
#[PrimeFieldModulus = "340282366920938463463374607431768211297"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
struct FieldElement([u64; 3]);
impl FieldElement {
	/// Create a field element from a u64
	fn new(v: u64) -> Self {
		let mut bytes = [0u8; 3 * 8];
		bytes[0..8].copy_from_slice(&v.to_le_bytes());
		let repr = FieldElementRepr(bytes);
		let elm: FieldElement = PrimeField::from_repr(repr).expect("can create field elm from u64");
		elm
	}

	/// Rejection samples random field element, we only sample 128bit since modulus is 128bit
	fn random() -> Self {
		let mut rng = rand::thread_rng();
		let mut bytes = [0u8; 3 * 8];
		let max_fill = 2 * 8;
		loop {
			rng.fill_bytes(&mut bytes[0..max_fill]);
			let repr = FieldElementRepr(bytes);
			if let Some(e) = PrimeField::from_repr(repr) {
				return e;
			}
		}
	}
}

/// Represents a polynomial over the finite field
#[derive(Debug)]
struct Polynomial {
	degree: u64,
	coefficients: Vec<FieldElement>,
}

/// Represents a point on the polynomial
#[derive(Debug)]
struct Share {
	x: FieldElement,
	y: FieldElement,
}

impl Polynomial {
	/// Create random degree t-1 polynomial with f(0)=s
	fn new(t: u64, s: FieldElement) -> Self {
		let mut coef = vec![s];
		for _ in 1..t - 1 {
			let fe = FieldElement::random();
			coef.push(fe);
		}
		coef.reverse();

		Polynomial {
			degree: t - 1,
			coefficients: coef,
		}
	}

	/// Evaluate polynomial at f(x)
	fn evaluate(&self, x: &FieldElement) -> FieldElement {
		let mut result = self.coefficients[0];
		for i in 1..self.degree as usize {
			result = result * x + self.coefficients[i];
		}
		result
	}

	/// Evaluate polynomial at f(1), .., f(n)
	fn share(&self, n: u64) -> Vec<Share> {
		let mut shares = Vec::new();
		for i in 1..=n {
			let x = FieldElement::new(i);
			let y = self.evaluate(&x);
			shares.push(Share { x, y })
		}
		shares
	}

	/// Compute f(0) by interpolation
	fn reconstruct(shares: &[Share]) -> FieldElement {
		// how do I use a closure?
		// let lagrange_basis_eval = |j: usize, x: FieldElement| unimplemented!();
		let num_keys = shares.len();
		let mut val = FieldElement::zero();
		for i in 0..num_keys - 1 {
			let y = shares[i].y;
			let mut d = FieldElement::one();
			let mut n = FieldElement::one();
			for j in 0..num_keys - 1 {
				if i != j {
					d = d * (-shares[j].x);
					n = n * (shares[i].x - shares[j].x);
				}
			}
			val = val + y * d * n.invert().unwrap();
		}
		val
	}
}

pub fn main() {
	let secret = FieldElement::new(42);

	println!("secret: {:?}", secret);

	println!("Creating new polynomial");
	let poly = Polynomial::new(5, secret.clone());

	println!("Sharing polynomial");
	let shares = poly.share(10);

	println!("Reconstructing polynomial");
	let interpolated = Polynomial::reconstruct(&shares[3..8]);

	println!("Checking secret is correct:");
	println!(
		"  {}",
		if interpolated == secret {
			"YAY HURRAY"
		} else {
			"NOPE TRY AGAIN"
		}
	);
}
