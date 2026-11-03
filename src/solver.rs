use std::{cmp::Ordering, collections::BinaryHeap};

struct Candidate<Domain> {
    max_bound: f64,
    domain: Domain,
}

impl<Domain> PartialEq for Candidate<Domain> {
    fn eq(&self, other: &Self) -> bool {
        self.max_bound == other.max_bound
    }
}

impl<Domain> Eq for Candidate<Domain> {}

impl<Domain> PartialOrd for Candidate<Domain> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Domain> Ord for Candidate<Domain> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.max_bound.partial_cmp(&other.max_bound).unwrap()
    }
}

pub fn solve<Domain>(
    domain: Domain,
    mut calc_max_bound: impl FnMut(&Domain, Option<f64>) -> f64,
    mut do_split: impl FnMut(&Domain) -> (Domain, Domain),
    mut is_good_enough: impl FnMut(&Domain) -> bool,
    mut progress: impl FnMut(f64),
) -> (f64, Domain) {
    let mut space = BinaryHeap::from([Candidate {
        max_bound: calc_max_bound(&domain, None),
        domain,
    }]);

    loop {
        let Candidate { max_bound, domain } = space.pop().unwrap();
        progress(max_bound);
        if is_good_enough(&domain) {
            return (max_bound, domain);
        }

        let (a, b) = do_split(&domain);
        space.push(Candidate {
            max_bound: calc_max_bound(&a, Some(max_bound)),
            domain: a,
        });
        space.push(Candidate {
            max_bound: calc_max_bound(&b, Some(max_bound)),
            domain: b,
        });
    }
}
