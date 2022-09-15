use std::{
    cmp,
    io::{self, BufRead},
    str::FromStr,
};

use once_cell::sync::OnceCell;
use regex::Regex;

/// All extractable data from a single micro-benchmark.
#[derive(Clone, Debug)]
pub struct Benchmark {
    /// e.g. mod::test_name
    pub name: String,
    /// e.g. test_name
    pub shortname: String,
    /// The benchmarks duration
    pub ns: u64,
    /// The benchmarks variance
    pub variance: u64,
    /// Throughput of the benchmark if available
    pub throughput: Option<u64>,
}

impl Eq for Benchmark {}

impl PartialEq for Benchmark {
    fn eq(&self, other: &Benchmark) -> bool {
        self.name == other.name
    }
}

impl Ord for Benchmark {
    fn cmp(&self, other: &Benchmark) -> cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Benchmark {
    fn partial_cmp(&self, other: &Benchmark) -> Option<cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

fn get_benchmark_regex() -> &'static Regex {
    static INSTANCE: OnceCell<Regex> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        Regex::new(
            r##"(?x)
        test\s+(?P<name>\S+)                        # test   mod::test_name
        \s+...\sbench:\s+(?P<ns>[0-9,]+)\s+ns/iter  # ... bench: 1234 ns/iter
        \s+\(\+/-\s+(?P<variance>[0-9,]+)\)         # (+/- 4321)
        (?:\s+=\s+(?P<throughput>[0-9,]+)\sMB/s)?   # =   2314 MB/s
    "##,
        )
        .unwrap()
    })
}

impl FromStr for Benchmark {
    type Err = ();

    /// Parses a single benchmark line into a Benchmark.
    fn from_str(line: &str) -> Result<Benchmark, ()> {
        let caps = match get_benchmark_regex().captures(line) {
            None => return Err(()),
            Some(caps) => caps,
        };
        let ns = match parse_commas(&caps["ns"]) {
            None => return Err(()),
            Some(ns) => ns,
        };
        let variance = match parse_commas(&caps["variance"]) {
            None => return Err(()),
            Some(variance) => variance,
        };
        let throughput = caps
            .name("throughput")
            .and_then(|m| parse_commas(m.as_str()));
        let name = caps["name"].to_string();
        let shortname = (&name)
            .rsplit_once(':')
            .map(|el| el.1)
            .unwrap_or(&name)
            .to_string();
        Ok(Benchmark {
            name,
            shortname,
            ns,
            variance,
            throughput,
        })
    }
}

/// Drops all commas in a string and parses it as a unsigned integer
fn parse_commas(s: &str) -> Option<u64> {
    drop_commas(s).parse().ok()
}

/// Drops all commas in a string
fn drop_commas(s: &str) -> String {
    s.chars().filter(|&b| b != ',').collect()
}

/// Parse benchmarks from a buffered reader.
pub fn parse_lines<B: BufRead>(buffer: B) -> io::Result<Vec<Benchmark>> {
    let iter = buffer.lines();
    let mut vec = Vec::with_capacity(iter.size_hint().0);
    for result in iter {
        if let Ok(bench) = result?.parse() {
            vec.push(bench)
        }
    }
    Ok(vec)
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    const TEST_DATA: &str = r#"running 3 tests
test fastfield::multivalued::bench::bench_multi_value_ff_creation                                                        ... bench:  95,653,541 ns/iter (+/- 1,410,738)
test fastfield::multivalued::bench::bench_multi_value_ff_creation_with_sorting                                           ... bench: 103,466,980 ns/iter (+/- 6,247,651)
test fastfield::multivalued::bench::bench_multi_value_fflookup                                                           ... bench:   1,330,510 ns/iter (+/- 217,966)"#;

    #[test]
    fn parse_bench_output_test() {
        let reader = BufReader::new(TEST_DATA.as_bytes());
        let benchmarks = parse_lines(reader).unwrap();

        let shortnames: Vec<_> = benchmarks.iter().map(|bench| &bench.shortname).collect();
        assert_eq!(
            shortnames,
            &[
                "bench_multi_value_ff_creation",
                "bench_multi_value_ff_creation_with_sorting",
                "bench_multi_value_fflookup"
            ]
        );

        let shortnames: Vec<_> = benchmarks.iter().map(|bench| bench.ns).collect();
        assert_eq!(shortnames, &[95653541, 103466980, 1330510]);
    }
}
