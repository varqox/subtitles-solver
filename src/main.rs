use std::cmp::{max, min};

use log::{LevelFilter, debug, info, warn};

mod decoders;
mod encoders;
mod solver;

#[derive(Debug)]
struct Subtitles(Vec<Subtitle>);

#[derive(Debug)]
struct Subtitle {
    beg: f64,
    end: f64,
    text: String,
}

pub mod cli {
    use std::path::PathBuf;

    use clap::{Parser, Subcommand};

    #[derive(Parser, Debug)]
    #[command(
        name = "subtitles-solver",
        version,
        about = "Synchronize subtitles to match times of another subtitles with math and algorithms"
    )]
    pub struct Args {
        /// Print debug logs
        #[arg(short, long)]
        pub debug: bool,
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Synchronize subtitles to another subtitles
        Sync {
            /// Subtitles file to synchronize with
            #[arg(long)]
            to: PathBuf,
            /// Subtitles that will be synchronized
            input: PathBuf,
            /// Output subtitle file
            out: PathBuf,
        },
    }
}

fn init_logging(debug: bool) {
    let mut logger = pretty_env_logger::formatted_builder();
    logger.parse_default_env();
    if debug {
        logger.filter_level(LevelFilter::Debug);
    }
    logger.init();
}

#[derive(Clone, Copy, Debug)]
struct Range {
    beg: f64,
    end: f64,
}

impl Range {
    fn len(&self) -> f64 {
        self.end - self.beg
    }

    fn midpoint(&self) -> f64 {
        self.beg.midpoint(self.end)
    }
}

#[derive(Clone, Debug)]
struct RectDomain {
    shift: Range,
    stretch: Range,
}

// Calculate maximal value of |A \cap to|/min(|A|, |to|), where A = from * y + x, for y \in domain.stretch, x \in domain.shift
fn max_match_in(domain: &RectDomain, from: &Range, to: &Range) -> f64 {
    let a = from.beg;
    let b = from.end;
    let c = to.beg;
    let d = to.end;
    debug_assert!(a < b);
    debug_assert!(c < d);
    debug_assert!(0. <= a);
    debug_assert!(0. <= c);
    debug_assert!(domain.stretch.beg <= domain.stretch.end);
    debug_assert!(domain.shift.beg <= domain.shift.end);
    debug_assert!(domain.stretch.beg > 0.);

    // We only need to check two points: BL and TR. This is not obvious, but going if BL and TR
    // belong to the same region where 0 < score < 1, only those two can have maximum value.
    //    +------+ TR
    //    |      |
    //    |      |
    // BL +------+
    let bl_x = domain.shift.beg;
    let bl_y = domain.stretch.beg;
    let tr_x = domain.shift.end;
    let tr_y = domain.stretch.end;
    // L(y) = c - y*b  <  R(y) = d - y*a
    // ^ Left bound    <  ^ Right bound
    let l_bl = c - bl_y * b;
    let l_tr = c - tr_y * b;
    let r_bl = d - bl_y * a;
    let r_tr = d - tr_y * a;
    // Let bisector of L and R be: Bi(y) = (L(y) + R(y)) / 2
    // Now we calculate on which side of the bisector is BL and TR
    let bl_dx_bi = bl_x - (l_bl + r_bl) / 2.;
    let tr_dx_bi = tr_x - (l_tr + r_tr) / 2.;
    if bl_dx_bi <= 0. && tr_dx_bi >= 0. {
        return 1.; // (BL, TR) crosses Bi(y), and on Bi(y) score is 1 (maximum).
    }
    if tr_dx_bi < 0. {
        // Under Bi -> TR is better.
        let tr_dx_l = tr_x - l_tr; // TR.x - L(TR.y)
        if tr_dx_l <= 0. {
            return 0.;
        }
        let den = (d - c).min(tr_y * (b - a));
        (tr_dx_l / den).min(1.)
    } else {
        // Above Bi -> BL is better.
        let bl_dx_r = r_bl - bl_x;
        if bl_dx_r <= 0. {
            return 0.;
        }
        let den = (d - c).min(bl_y * (b - a));
        (bl_dx_r / den).min(1.)
    }
}

// struct Scorer<'f, 't> {
//     dist: Vec<f64>,
//     diagonals: Vec<usize>, // we have guards at each end
//     from: &'f [Range],
//     to: &'t [Range],
// }

// impl<'f, 't> Scorer<'f, 't> {
//     fn new(from: &'f [Range], to: &'t [Range]) -> Self {
//         let mut dist = vec![0.; (from.len() + 1) * (to.len() + 1)];
//         // Fill sentinels
//         //             from
//         //    0 | 1 | 2 | 3 | 4 | 5 |
//         //   ---+---+---+---+---+---+
//         //    1 |   |   |   |   |   |
//         //   ---+---+---+---+---+---+
//         //    2 |   |   |   |   |   |
//         //   ---+---+---+---+---+---+
//         // to 3 |   |   |   |   |   |
//         //   ---+---+---+---+---+---+
//         //    4 |   |   |   |   |   |
//         //   ---+---+---+---+---+---+
//         //    5 |   |   |   |   |   |
//         //   ---+---+---+---+---+---+
//         for i in 1..=from.len() {
//             dist[i] = i as f64;
//         }
//         for j in 1..=to.len() {
//             dist[j * (from.len() + 1)] = j as f64;
//         }
//         Self {
//             dist,
//             diagonals: vec![0; from.len() + to.len() + 1],
//             from,
//             to,
//         }
//     }

//     fn max_score_in(&mut self, domain: &RectDomain) -> f64 {
//         if self.from.is_empty() || self.to.is_empty() {
//             return 0.;
//         }

//         macro_rules! dp {
//             ($i: expr, $j:expr) => {
//                 self.dist[($i) * (self.from.len() + 1) + ($j)]
//             };
//         }

//         //         from
//         //    +---------------+
//         //    |\              |
//         // to | \ central     |
//         //    |  \            |
//         //    +---------------+

//         // Index of the central diagonal starts at dp[1][1]
//         let central = self.to.len();
//         dp!(1, 1) = 2. - 2. * max_match_in(domain, &self.from[0], &self.to[0]);
//         self.diagonals[central] = 0;
//         let mut bound = 0;
//         loop {
//             bound += 1;
//             // Now, calculate all cells where dp[*][*] <= bound.

//             // diagonals[central + i] == k means that on that diagonal,
//             //   dp[k][i + k] is calculated and dp[k][i + k] <= bound - 1
//             // diagonals[central - i] == k means that on that diagonal,
//             //   dp[i + k][k] is calculated and dp[i + k][k] <= bound - 1

//             // Fill in sentinel diagonals
//             if bound <= self.from.len() {
//                 self.diagonals[central + bound] = 0;
//             }
//             if bound < self.from.len() {
//                 dp!(1, 1 + bound) =
//                     dp!(0, bound) + 2. - 2. * max_match_in(domain, &self.from[bound], &self.to[0]);
//             }
//             if bound <= self.to.len() {
//                 self.diagonals[central - bound] = 0;
//             }
//             if bound < self.to.len() {
//                 dp!(1 + bound, 1) =
//                     dp!(bound, 0) + 2. - 2. * max_match_in(domain, &self.from[0], &self.to[bound]);
//             }

//             // Central diagonal first
//             let mut i = self.diagonals[central];
//             let prev_diagonals_center = i;
//             if i < min(self.to.len(), self.from.len()) {
//                 loop {
//                     // dp[i][i] is calculated,
//                     // diagonal for dp[i + 1][i + 1] is already calculated.
//                     // But carrying from neighbor diagonals may produce a better score.
//                     if self.diagonals[central + 1] >= i {
//                         dp!(i + 1, i + 1) = dp!(i + 1, i + 1).min(dp!(i, i + 1) + 1.);
//                     }
//                     if self.diagonals[central - 1] >= i {
//                         dp!(i + 1, i + 1) = dp!(i + 1, i + 1).min(dp!(i + 1, i) + 1.);
//                     }
//                     if dp!(i + 1, i + 1) > bound as f64 {
//                         break;
//                     }
//                     i += 1;
//                     if i == min(self.to.len(), self.from.len()) {
//                         break;
//                     }
//                     dp!(i + 1, i + 1) =
//                         dp!(i, i) + 2. - 2. * max_match_in(domain, &self.from[i], &self.to[i]);
//                 }
//             }
//             self.diagonals[central] = i;

//             // Up diagonals
//             let mut prev_diagonals_k_minus_1 = prev_diagonals_center;
//             for k in 1..min(self.from.len(), bound) {
//                 i = self.diagonals[central + k];
//                 let new_prev_diagonals_k_minus_1 = i;
//                 if i < min(self.to.len(), self.from.len() - k) {
//                     loop {
//                         // dp[i][i + k] is calculated,
//                         // diagonal for dp[i + 1][i + k + 1] is already calculated.
//                         if self.diagonals[central + k + 1] >= i {
//                             dp!(i + 1, i + k + 1) =
//                                 dp!(i + 1, i + k + 1).min(dp!(i, i + k + 1) + 1.);
//                         }
//                         if prev_diagonals_k_minus_1 >= i {
//                             dp!(i + 1, i + k + 1) =
//                                 dp!(i + 1, i + k + 1).min(dp!(i + 1, i + k) + 1.);
//                         }
//                         if dp!(i + 1, i + k + 1) > bound as f64 {
//                             break;
//                         }
//                         i += 1;
//                         if i == min(self.to.len(), self.from.len() - k) {
//                             break;
//                         }
//                         dp!(i + 1, i + k + 1) = dp!(i, i + k) + 2.
//                             - 2. * max_match_in(domain, &self.from[i + k], &self.to[i]);
//                     }
//                 }
//                 self.diagonals[central + k] = i;
//                 prev_diagonals_k_minus_1 = new_prev_diagonals_k_minus_1;
//             }

//             let mut prev_diagonals_k_plus_1 = prev_diagonals_center;
//             for k in 1..min(self.to.len(), bound) {
//                 i = self.diagonals[central - k];
//                 let new_prev_diagonals_k_plus_1 = i;
//                 if i < min(self.to.len() - k, self.from.len()) {
//                     loop {
//                         // dp[i + k][i] is calculated,
//                         // diagonal for dp[i + k + 1][i + 1] is already calculated.
//                         if prev_diagonals_k_plus_1 >= i {
//                             dp!(i + k + 1, i + 1) =
//                                 dp!(i + k + 1, i + 1).min(dp!(i + k, i + 1) + 1.);
//                         }
//                         if self.diagonals[central - k - 1] >= i {
//                             dp!(i + k + 1, i + 1) =
//                                 dp!(i + k + 1, i + 1).min(dp!(i + k + 1, i) + 1.);
//                         }
//                         if dp!(i + k + 1, i + 1) > bound as f64 {
//                             break;
//                         }
//                         i += 1;
//                         if i == min(self.to.len() - k, self.from.len()) {
//                             break;
//                         }
//                         dp!(i + k + 1, i + 1) = dp!(i + k, i) + 2.
//                             - 2. * max_match_in(domain, &self.from[i], &self.to[i + k]);
//                     }
//                 }
//                 self.diagonals[central - k] = i;
//                 prev_diagonals_k_plus_1 = new_prev_diagonals_k_plus_1;
//             }

//             let diff = max(self.to.len(), self.from.len()) - min(self.to.len(), self.from.len());
//             if bound >= diff
//                 && self.diagonals[central + self.from.len() - self.to.len()]
//                     == min(self.to.len(), self.from.len())
//             {
//                 return ((self.to.len() + self.from.len()) as f64
//                     - dp!(self.to.len(), self.from.len()))
//                     / 2.;
//             }
//         }
//     }
// }

// fn max_score_in_orig(domain: &RectDomain, from: &[Range], to: &[Range]) -> f64 {
//     if from.is_empty() || to.is_empty() {
//         return 0.;
//     }
//     let mut dp = vec![0_f64; from.len()];
//     for r in to {
//         let mut diag = dp[0];
//         dp[0] = dp[0].max(max_match_in(domain, &from[0], r));
//         for i in 1..from.len() {
//             let next_diag = dp[i];
//             dp[i] = dp[i]
//                 .max(dp[i - 1])
//                 .max(diag + max_match_in(domain, &from[i], r));
//             diag = next_diag;
//         }
//     }
//     dp[from.len() - 1]
// }

#[allow(unused)]
fn max_score_in_general_optimized_double_lcs(
    domain: &RectDomain,
    from: &[Range],
    to: &[Range],
    max_score_upper_bound: Option<f64>,
) -> f64 {
    if from.is_empty() || to.is_empty() {
        return 0.;
    }
    let (l, r) = if to.len() < from.len() {
        (0, from.len() - to.len())
    } else {
        (to.len() - from.len(), 0)
    };
    let min_len = min(from.len(), to.len());
    let mut k = max(
        min_len - max_score_upper_bound.unwrap_or(min_len as f64) as usize,
        (l + r) / 3,
    ) * 4
        / 3
        + 2;
    let mut dp = vec![0_f64; from.len() + 1];
    loop {
        dp.fill(0.);
        for (j, elem) in to.iter().enumerate() {
            let start = j.saturating_sub(l + k);
            let end = min(j + r + k, from.len());
            let mut diag = dp[start];
            for i in start..end {
                let next_diag = dp[i + 1];
                dp[i + 1] = dp[i + 1].max(dp[i]);
                if diag + 1.0 > dp[i + 1] {
                    dp[i + 1] = dp[i + 1].max(diag + max_match_in(domain, &from[i], elem));
                }

                diag = next_diag;
            }
        }
        let score = dp[from.len()];
        if score >= min_len.saturating_sub(k) as f64 {
            return score;
        }
        k *= 2;
    }
}

fn max_score_in_specialized_double_lcs(
    domain: &RectDomain,
    from: &[Range],
    to: &[Range],
    max_score_upper_bound: Option<f64>,
) -> f64 {
    if from.is_empty() || to.is_empty() {
        return 0.;
    }
    let (l, r) = if to.len() < from.len() {
        (0, from.len() - to.len())
    } else {
        (to.len() - from.len(), 0)
    };
    let min_len = min(from.len(), to.len());
    let mut k = max(
        min_len
            - max_score_upper_bound
                .unwrap_or(min_len as f64)
                .min(min_len as f64) as usize,
        (l + r) / 3,
    ) * 4
        / 3
        + 2;
    let mut dp = vec![0_f64; from.len() + 1];
    loop {
        dp.fill(0.);
        let mut non_zero_score_beg = 0;
        let mut non_zero_score_end = 0;
        let mut explored_more = false;
        for (j, elem) in to.iter().enumerate() {
            while non_zero_score_beg < from.len() {
                let f = &from[non_zero_score_beg];
                let end = f.end * domain.stretch.end + domain.shift.end;
                if end <= elem.beg {
                    dp[non_zero_score_beg + 1] =
                        dp[non_zero_score_beg + 1].max(dp[non_zero_score_beg]);
                    non_zero_score_beg += 1;
                } else {
                    break;
                }
            }
            non_zero_score_end = non_zero_score_end.max(non_zero_score_beg);
            while non_zero_score_end < from.len() {
                let f = &from[non_zero_score_end];
                let beg = f.beg * domain.stretch.beg + domain.shift.beg;
                if beg < elem.end {
                    dp[non_zero_score_end + 1] =
                        dp[non_zero_score_end + 1].max(dp[non_zero_score_end]);
                    non_zero_score_end += 1;
                } else {
                    break;
                }
            }
            let start = j.saturating_sub(l + k);
            let end = min(j + r + k, from.len());
            if non_zero_score_beg < non_zero_score_end
                && (start > non_zero_score_beg || end < non_zero_score_end)
            {
                explored_more = true;
            }
            let start = start.max(non_zero_score_beg);
            let end = end.min(non_zero_score_end);
            let mut diag = dp[start];
            for i in start..end {
                let next_diag = dp[i + 1];
                dp[i + 1] = dp[i + 1].max(dp[i]);
                if diag + 1.0 > dp[i + 1] {
                    dp[i + 1] = dp[i + 1].max(diag + max_match_in(domain, &from[i], elem));
                }

                diag = next_diag;
            }
        }
        let score = dp[non_zero_score_end];
        if score >= min_len.saturating_sub(k) as f64 || !explored_more {
            return score;
        }
        k *= 2;
    }
}

fn main() -> anyhow::Result<()> {
    let args = <cli::Args as clap::Parser>::parse();
    init_logging(args.debug);

    let cli::Commands::Sync { to, input, out } = args.command;
    let to_subtitles = decoders::decode_subtitles_file(to)?;
    let input_subtitles = decoders::decode_subtitles_file(input)?;
    let mut from_ranges: Vec<_> = input_subtitles
        .0
        .iter()
        .map(|s| Range {
            beg: s.beg,
            end: s.end,
        })
        .collect();
    let mut to_ranges: Vec<_> = to_subtitles
        .0
        .iter()
        .map(|s| Range {
            beg: s.beg,
            end: s.end,
        })
        .collect();
    // Shift ranges so that every point >= 0.
    let mut min = from_ranges.first().map(|s| s.beg).unwrap_or(0.);
    let mut max = 0_f64;
    for s in from_ranges.iter().chain(to_ranges.iter()) {
        min = min.min(s.beg.min(s.end));
        max = max.max(s.beg.max(s.end));
    }
    for s in from_ranges.iter_mut().chain(to_ranges.iter_mut()) {
        s.beg -= min;
        s.end -= min;
    }
    max -= min;
    let calc_max_bound = |domain: &RectDomain, max_score_upper_bound: Option<f64>| {
        max_score_in_specialized_double_lcs(domain, &from_ranges, &to_ranges, max_score_upper_bound)
    };
    let do_split = |RectDomain { shift, stretch }: &RectDomain| {
        if shift.len() > stretch.len() * max {
            let mid = shift.midpoint();
            (
                RectDomain {
                    shift: Range {
                        beg: shift.beg,
                        end: mid,
                    },
                    stretch: *stretch,
                },
                RectDomain {
                    shift: Range {
                        beg: mid,
                        end: shift.end,
                    },
                    stretch: *stretch,
                },
            )
        } else {
            let mid = stretch.midpoint();
            (
                RectDomain {
                    shift: *shift,
                    stretch: Range {
                        beg: stretch.beg,
                        end: mid,
                    },
                },
                RectDomain {
                    shift: *shift,
                    stretch: Range {
                        beg: mid,
                        end: stretch.end,
                    },
                },
            )
        }
    };
    let is_good_enough =
        |RectDomain { shift, stretch }: &RectDomain| shift.len() + stretch.len() * max < 0.001;

    let progress_bar = indicatif::ProgressBar::no_length()
        .with_style(indicatif::ProgressStyle::with_template("{msg}")?);
    let inv_longer_len = 100. / from_ranges.len().max(to_ranges.len()) as f64;
    let progress = |max_bound| {
        progress_bar.set_message(format!(
            "Optimal match <= {:.3}%",
            max_bound * inv_longer_len
        ));
    };
    let (res_bound, res_domain) = solver::solve(
        RectDomain {
            shift: Range {
                beg: -4. * max,
                end: max,
            },
            stretch: Range { beg: 0.25, end: 4. },
        },
        calc_max_bound,
        do_split,
        is_good_enough,
        progress,
    );
    let shift = res_domain.shift.midpoint();
    let stretch = res_domain.stretch.midpoint();
    progress_bar.finish_with_message(format!(
        "Optimal match ~ {:.3}%",
        calc_max_bound(
            &RectDomain {
                shift: Range {
                    beg: shift,
                    end: shift
                },
                stretch: Range {
                    beg: stretch,
                    end: stretch
                }
            },
            Some(res_bound)
        ) * inv_longer_len
    ));
    debug!("res {{ bound: {}, domain: {:?} }}", res_bound, res_domain);
    info!("stretch: {:.9}, shift: {:.4}", stretch, shift);

    let out_subtitles = Subtitles(
        input_subtitles
            .0
            .into_iter()
            .filter_map(|Subtitle { beg, end, text }| {
                let beg = (beg - min) * stretch + shift + min;
                let end = (end - min) * stretch + shift + min;
                if end <= 0. {
                    warn!(
                        "Dropping subtitles with negative time: {:.3}s -> {:.3}s, text: {}",
                        beg, end, text
                    );
                    None
                } else {
                    if beg < 0. {
                        warn!(
                        "Clamping subtitles that begin < 0s to start at 0: {:.3}s -> {:.3}s, text: {}", beg, end, text);

                    }
                    Some(Subtitle { beg: beg.max(0.), end, text })
                }
            })
            .collect(),
    );
    encoders::encode_subtitles_to_file(&out_subtitles, out)?;
    Ok(())
}
