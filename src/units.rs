//! Unit conversion tables and scientific constants.

pub struct Unit {
    pub name: &'static str,
    pub symbol: &'static str,
    /// base = value × factor + offset
    pub factor: f64,
    pub offset: f64,
}

pub struct Category {
    pub name: &'static str,
    pub units: &'static [Unit],
}

const fn u(name: &'static str, symbol: &'static str, factor: f64) -> Unit {
    Unit { name, symbol, factor, offset: 0.0 }
}

/// Look a unit up by symbol (exact) or name (any case, singular or plural): (category index, unit).
pub fn find(text: &str) -> Option<(usize, &'static Unit)> {
    let t = text.trim().trim_end_matches(['.', ',']);
    if t.is_empty() {
        return None;
    }
    for (ci, c) in CATEGORIES.iter().enumerate() {
        if let Some(u) = c.units.iter().find(|u| u.symbol == t) {
            return Some((ci, u));
        }
    }
    let lower = t.to_lowercase();
    let singular = lower.strip_suffix('s').unwrap_or(&lower);
    for (ci, c) in CATEGORIES.iter().enumerate() {
        if let Some(u) = c.units.iter().find(|u| {
            let n = u.name.to_lowercase();
            n == lower || n == singular || n.strip_suffix('s').unwrap_or(&n) == singular || u.symbol.to_lowercase() == lower
        }) {
            return Some((ci, u));
        }
    }
    None
}

pub fn convert(value: f64, from: &Unit, to: &Unit) -> f64 {
    (value * from.factor + from.offset - to.offset) / to.factor
}

pub const CATEGORIES: &[Category] = &[
    Category {
        name: "Length",
        units: &[
            u("Metre", "m", 1.0),
            u("Kilometre", "km", 1000.0),
            u("Centimetre", "cm", 0.01),
            u("Millimetre", "mm", 0.001),
            u("Micrometre", "µm", 1e-6),
            u("Nanometre", "nm", 1e-9),
            u("Inch", "in", 0.0254),
            u("Foot", "ft", 0.3048),
            u("Yard", "yd", 0.9144),
            u("Mile", "mi", 1609.344),
            u("Nautical mile", "nmi", 1852.0),
            u("Light-year", "ly", 9.4607304725808e15),
        ],
    },
    Category {
        name: "Area",
        units: &[
            u("Square metre", "m²", 1.0),
            u("Square kilometre", "km²", 1e6),
            u("Square centimetre", "cm²", 1e-4),
            u("Hectare", "ha", 1e4),
            u("Square inch", "in²", 0.00064516),
            u("Square foot", "ft²", 0.09290304),
            u("Square yard", "yd²", 0.83612736),
            u("Acre", "ac", 4046.8564224),
            u("Square mile", "mi²", 2589988.110336),
        ],
    },
    Category {
        name: "Volume",
        units: &[
            u("Litre", "L", 0.001),
            u("Millilitre", "mL", 1e-6),
            u("Cubic metre", "m³", 1.0),
            u("Cubic centimetre", "cm³", 1e-6),
            u("Cubic inch", "in³", 1.6387064e-5),
            u("Cubic foot", "ft³", 0.028316846592),
            u("US gallon", "gal", 0.003785411784),
            u("US quart", "qt", 0.000946352946),
            u("US pint", "pt", 0.000473176473),
            u("US cup", "cup", 0.0002365882365),
            u("US fluid ounce", "fl oz", 2.95735295625e-5),
            u("Imperial gallon", "imp gal", 0.00454609),
        ],
    },
    Category {
        name: "Mass",
        units: &[
            u("Kilogram", "kg", 1.0),
            u("Gram", "g", 0.001),
            u("Milligram", "mg", 1e-6),
            u("Tonne", "t", 1000.0),
            u("Ounce", "oz", 0.028349523125),
            u("Pound", "lb", 0.45359237),
            u("Stone", "st", 6.35029318),
            u("US ton", "ton", 907.18474),
            u("Carat", "ct", 0.0002),
        ],
    },
    Category {
        name: "Temperature",
        units: &[
            Unit { name: "Celsius", symbol: "°C", factor: 1.0, offset: 273.15 },
            Unit { name: "Fahrenheit", symbol: "°F", factor: 5.0 / 9.0, offset: 459.67 * 5.0 / 9.0 },
            Unit { name: "Kelvin", symbol: "K", factor: 1.0, offset: 0.0 },
        ],
    },
    Category {
        name: "Speed",
        units: &[
            u("Metre per second", "m/s", 1.0),
            u("Kilometre per hour", "km/h", 1.0 / 3.6),
            u("Mile per hour", "mph", 0.44704),
            u("Foot per second", "ft/s", 0.3048),
            u("Knot", "kn", 1852.0 / 3600.0),
        ],
    },
    Category {
        name: "Time",
        units: &[
            u("Second", "s", 1.0),
            u("Millisecond", "ms", 0.001),
            u("Microsecond", "µs", 1e-6),
            u("Minute", "min", 60.0),
            u("Hour", "h", 3600.0),
            u("Day", "d", 86400.0),
            u("Week", "wk", 604800.0),
            u("Year (365 d)", "yr", 31536000.0),
        ],
    },
    Category {
        name: "Pressure",
        units: &[
            u("Pascal", "Pa", 1.0),
            u("Kilopascal", "kPa", 1000.0),
            u("Bar", "bar", 1e5),
            u("Atmosphere", "atm", 101325.0),
            u("Millimetre of mercury", "mmHg", 133.322387415),
            u("Pound per square inch", "psi", 6894.757293168),
        ],
    },
    Category {
        name: "Energy",
        units: &[
            u("Joule", "J", 1.0),
            u("Kilojoule", "kJ", 1000.0),
            u("Calorie", "cal", 4.184),
            u("Kilocalorie", "kcal", 4184.0),
            u("Watt-hour", "Wh", 3600.0),
            u("Kilowatt-hour", "kWh", 3.6e6),
            u("Electronvolt", "eV", 1.602176634e-19),
            u("British thermal unit", "BTU", 1055.05585262),
        ],
    },
    Category {
        name: "Power",
        units: &[
            u("Watt", "W", 1.0),
            u("Kilowatt", "kW", 1000.0),
            u("Megawatt", "MW", 1e6),
            u("Horsepower (mech.)", "hp", 745.69987158227),
            u("Horsepower (metric)", "PS", 735.49875),
        ],
    },
    Category {
        name: "Force",
        units: &[
            u("Newton", "N", 1.0),
            u("Kilonewton", "kN", 1000.0),
            u("Kilogram-force", "kgf", 9.80665),
            u("Pound-force", "lbf", 4.4482216152605),
            u("Dyne", "dyn", 1e-5),
        ],
    },
    Category {
        name: "Angle",
        units: &[
            u("Degree", "°", std::f64::consts::PI / 180.0),
            u("Radian", "rad", 1.0),
            u("Gradian", "grad", std::f64::consts::PI / 200.0),
            u("Turn", "rev", 2.0 * std::f64::consts::PI),
            u("Arcminute", "′", std::f64::consts::PI / 10800.0),
            u("Arcsecond", "″", std::f64::consts::PI / 648000.0),
        ],
    },
    Category {
        name: "Data",
        units: &[
            u("Bit", "bit", 0.125),
            u("Byte", "B", 1.0),
            u("Kilobyte", "kB", 1e3),
            u("Megabyte", "MB", 1e6),
            u("Gigabyte", "GB", 1e9),
            u("Terabyte", "TB", 1e12),
            u("Kibibyte", "KiB", 1024.0),
            u("Mebibyte", "MiB", 1048576.0),
            u("Gibibyte", "GiB", 1073741824.0),
            u("Tebibyte", "TiB", 1099511627776.0),
        ],
    },
];

pub struct Constant {
    pub name: &'static str,
    pub symbol: &'static str,
    pub value: f64,
    pub unit: &'static str,
}

const fn k(name: &'static str, symbol: &'static str, value: f64, unit: &'static str) -> Constant {
    Constant { name, symbol, value, unit }
}

/// CODATA 2018 values.
pub const CONSTANTS: &[Constant] = &[
    k("Speed of light in vacuum", "c", 299792458.0, "m/s"),
    k("Planck constant", "h", 6.62607015e-34, "J·s"),
    k("Reduced Planck constant", "ħ", 1.054571817e-34, "J·s"),
    k("Gravitational constant", "G", 6.67430e-11, "m³/(kg·s²)"),
    k("Standard gravity", "g", 9.80665, "m/s²"),
    k("Elementary charge", "e", 1.602176634e-19, "C"),
    k("Avogadro constant", "Nₐ", 6.02214076e23, "1/mol"),
    k("Boltzmann constant", "k", 1.380649e-23, "J/K"),
    k("Molar gas constant", "R", 8.314462618, "J/(mol·K)"),
    k("Faraday constant", "F", 96485.33212, "C/mol"),
    k("Stefan–Boltzmann constant", "σ", 5.670374419e-8, "W/(m²·K⁴)"),
    k("Vacuum permittivity", "ε₀", 8.8541878128e-12, "F/m"),
    k("Vacuum permeability", "μ₀", 1.25663706212e-6, "N/A²"),
    k("Electron mass", "mₑ", 9.1093837015e-31, "kg"),
    k("Proton mass", "mₚ", 1.67262192369e-27, "kg"),
    k("Neutron mass", "mₙ", 1.67492749804e-27, "kg"),
    k("Atomic mass unit", "u", 1.66053906660e-27, "kg"),
    k("Bohr radius", "a₀", 5.29177210903e-11, "m"),
    k("Rydberg constant", "R∞", 10973731.568160, "1/m"),
    k("Fine-structure constant", "α", 7.2973525693e-3, ""),
    k("Standard atmosphere", "atm", 101325.0, "Pa"),
    k("Molar volume (ideal gas, STP)", "Vₘ", 22.41396954e-3, "m³/mol"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        let cat = |n: &str| CATEGORIES.iter().find(|c| c.name == n).unwrap();
        let unit = |c: &'static Category, s: &str| c.units.iter().find(|u| u.symbol == s).unwrap();
        let t = cat("Temperature");
        assert!((convert(100.0, unit(t, "°C"), unit(t, "°F")) - 212.0).abs() < 1e-9);
        assert!((convert(-40.0, unit(t, "°F"), unit(t, "°C")) + 40.0).abs() < 1e-9);
        assert!((convert(0.0, unit(t, "K"), unit(t, "°C")) + 273.15).abs() < 1e-9);
        let l = cat("Length");
        assert!((convert(1.0, unit(l, "mi"), unit(l, "km")) - 1.609344).abs() < 1e-12);
        let d = cat("Data");
        assert!((convert(1.0, unit(d, "GiB"), unit(d, "MiB")) - 1024.0).abs() < 1e-9);
    }
}
