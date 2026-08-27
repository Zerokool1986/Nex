use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianShare {
    pub guardian_index: u8,
    pub threshold: u8,
    pub total_shares: u8,
    pub epoch: u64,
    pub share_data: [u8; 32],
}

// GF(2^8) with irreducible polynomial 0x11B (x^8 + x^4 + x^3 + x + 1)
fn gf_add(a: u8, b: u8) -> u8 {
    a ^ b
}

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut res = 0u8;
    for _ in 0..8 {
        if (b & 1) != 0 {
            res ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1B; // Rijndael polynomial
        }
        b >>= 1;
    }
    res
}

fn gf_inv(mut a: u8) -> u8 {
    if a == 0 { return 0; }
    // In GF(2^8), a^254 = a^-1 (Fermat's Little Theorem)
    let mut res = 1u8;
    let mut base = a;
    let mut exp = 254;
    while exp > 0 {
        if (exp & 1) != 0 {
            res = gf_mul(res, base);
        }
        base = gf_mul(base, base);
        exp >>= 1;
    }
    res
}

fn gf_div(a: u8, b: u8) -> Result<u8, String> {
    if b == 0 {
        return Err("DivisionByZeroInGF".into());
    }
    Ok(gf_mul(a, gf_inv(b)))
}

/// Splits a 32-byte master root seed into N shares with threshold M over GF(2^8).
pub fn split_secret(
    secret: &[u8; 32],
    threshold: u8,
    total_shares: u8,
    epoch: u64,
    random_coefficients: &[Vec<u8>], // 32 vectors, each of length (threshold - 1)
) -> Result<Vec<GuardianShare>, String> {
    if threshold == 0 || threshold > total_shares {
        return Err("InvalidThreshold".into());
    }
    if total_shares == 0 {
        return Err("InvalidTotalShares".into());
    }
    if random_coefficients.len() != 32 {
        return Err("InvalidRandomCoeffsCount".into());
    }
    for coeffs in random_coefficients {
        if coeffs.len() != (threshold - 1) as usize {
            return Err("InvalidDegreeForCoeffs".into());
        }
    }

    let mut shares = Vec::new();
    for share_idx in 1..=total_shares {
        let mut share_data = [0u8; 32];
        for byte_idx in 0..32 {
            let secret_byte = secret[byte_idx];
            let mut y = secret_byte;
            let mut x_pow = share_idx;

            for coeff in &random_coefficients[byte_idx] {
                y = gf_add(y, gf_mul(*coeff, x_pow));
                x_pow = gf_mul(x_pow, share_idx);
            }
            share_data[byte_idx] = y;
        }

        shares.push(GuardianShare {
            guardian_index: share_idx,
            threshold,
            total_shares,
            epoch,
            share_data,
        });
    }

    Ok(shares)
}

/// Reconstructs the 32-byte master root seed from at least M guardian shares via Lagrange interpolation.
pub fn combine_shares(shares: &[GuardianShare], expected_threshold: u8) -> Result<[u8; 32], String> {
    if shares.len() < expected_threshold as usize {
        return Err("InsufficientSharesForQuorum".into());
    }

    let mut used_shares = Vec::new();
    let mut seen_indices = std::collections::HashSet::new();

    for share in shares {
        if seen_indices.insert(share.guardian_index) {
            used_shares.push(share);
            if used_shares.len() == expected_threshold as usize {
                break;
            }
        }
    }

    if used_shares.len() < expected_threshold as usize {
        return Err("DuplicateSharesFound".into());
    }

    let k = expected_threshold as usize;
    let mut secret = [0u8; 32];

    for byte_idx in 0..32 {
        let mut secret_byte = 0u8;

        for j in 0..k {
            let xj = used_shares[j].guardian_index;
            let yj = used_shares[j].share_data[byte_idx];

            // Compute Lagrange basis polynomial L_j(0) = product_{m != j} (xm / (xm - xj))
            let mut basis = 1u8;
            for m in 0..k {
                if m == j { continue; }
                let xm = used_shares[m].guardian_index;
                let num = xm;
                let den = gf_add(xm, xj); // xm - xj == xm ^ xj in GF(2^8)
                let factor = gf_div(num, den)?;
                basis = gf_mul(basis, factor);
            }

            secret_byte = gf_add(secret_byte, gf_mul(yj, basis));
        }

        secret[byte_idx] = secret_byte;
    }

    Ok(secret)
}
