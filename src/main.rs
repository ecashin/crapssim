use std::fmt;

use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use ndarray::Axis;
use ndarray_stats::{interpolate::Nearest, QuantileExt};
use noisy_float::types::n64;
use rand::{rngs::ThreadRng, Rng};

#[derive(Parser)]
struct Cli {
    #[clap(long)]
    n_trials: usize,
}

type Roll = (usize, usize);

fn roll(rng: &mut ThreadRng) -> Roll {
    let die1: usize = rng.gen_range(1..=6);
    let die2: usize = rng.gen_range(1..=6);
    (die1, die2)
}

#[derive(Clone, Debug)]
enum Bet {
    Pass(PassAttrs),
    Come(ComeAttrs),
}

#[derive(Clone)]
struct PassAttrs {
    amount: usize,
    odds: Option<usize>,
}

impl PassAttrs {
    fn new(amount: usize) -> Self {
        Self { amount, odds: None }
    }
}

impl fmt::Debug for PassAttrs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let odds = if let Some(o) = self.odds {
            format!("{o}")
        } else {
            "".to_string()
        };
        write!(f, "a{}o{}", self.amount, odds)
    }
}

#[derive(Clone)]
struct ComeAttrs {
    amount: usize,
    target: Option<usize>,
    odds: Option<usize>,
}

impl ComeAttrs {
    fn new(amount: usize) -> Self {
        Self {
            amount,
            target: None,
            odds: None,
        }
    }
}

impl fmt::Debug for ComeAttrs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let odds = if let Some(o) = self.odds {
            format!("{o}")
        } else {
            "".to_string()
        };
        let target = if let Some(t) = self.target {
            format!("{t}")
        } else {
            "".to_string()
        };
        write!(f, "a{}t{}o{}", self.amount, target, odds)
    }
}

fn odds_payout(amount: usize, target: usize) -> usize {
    let (numerator, denominator) = match target {
        4 | 10 => (2, 1),
        5 | 9 => (3, 2),
        6 | 8 => (5, 6),
        _ => panic!("What kind of odds bet was that!? {target}?"),
    };
    (amount * numerator) / denominator
}

fn odds_multiplier(target: usize) -> usize {
    match target {
        4 | 10 => 1,
        5 | 9 => 2,
        6 | 8 => 3,
        _ => panic!("What kind of odds bet was that!? {target}?"),
    }
}

fn main() -> Result<()> {
    simple_logger::init_with_env().context("setting up logging")?;
    let cli = Cli::parse();
    let mut rng = rand::thread_rng();
    let bet_min = 5;
    let mut roll_counts = vec![];
    for _ in 1..cli.n_trials {
        let n_rolls = one_scenario(&mut rng, bet_min);
        roll_counts.push(n_rolls);
    }
    println!("roll-count stats:");
    for quantile in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
        let mut a = ndarray::Array1::from_vec(roll_counts.clone());
        a.quantile_axis_mut(Axis(0), n64(quantile), &Nearest)
            .with_context(|| format!("computing quantile {quantile}"))?
            .for_each(|v| {
                let q = format!("q{quantile}");
                println!("{:>10}: {:>10?}", q, v);
            });
    }
    Ok(())
}

fn one_scenario(rng: &mut ThreadRng, bet_min: usize) -> usize {
    let mut bets = vec![Bet::Pass(PassAttrs::new(bet_min))];
    let mut point = None;
    let mut bankroll = 300 - bet_min;
    let mut i = 0;
    loop {
        i += 1;
        let mut new_bets = vec![];
        let dice = roll(rng);
        let sum = dice.0 + dice.1;
        info!("i:{i} roll:{dice:?} sum:{sum}");
        let mut new_point: Option<usize> = None;
        if sum == 7 {
            for bet in &bets {
                match bet {
                    Bet::Pass(PassAttrs { amount, odds: _ }) => {
                        if point.is_none() {
                            bankroll += 2 * amount;
                            info!("passline winner");
                        }
                    }
                    Bet::Come(ComeAttrs {
                        amount,
                        target,
                        odds: _,
                    }) => {
                        if target.is_none() {
                            bankroll += 2 * amount;
                            info!("come wins");
                        }
                    }
                }
            }
            new_point = None;
        } else {
            for bet in &bets {
                match bet {
                    Bet::Pass(PassAttrs { amount, odds }) => {
                        if let Some(p) = point {
                            if sum == p {
                                let mut winnings = 2 * amount;
                                if let Some(o) = odds {
                                    winnings += odds_payout(*o, p);
                                }
                                info!("pass wins {winnings} on point");
                                bankroll += winnings;
                            } else {
                                new_bets.push(bet.clone());
                            }
                            new_point = point;
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    bankroll += amount;
                                    info!("pass wins on yo");
                                }
                                _ => {
                                    new_point = Some(sum);
                                    let odds_amount = *amount * odds_multiplier(sum);
                                    let odds = if bankroll >= odds_amount {
                                        bankroll -= odds_amount;
                                        Some(odds_amount)
                                    } else {
                                        None
                                    };
                                    new_bets.push(Bet::Pass(PassAttrs {
                                        amount: *amount,
                                        odds,
                                    }));
                                }
                            }
                        }
                    }
                    Bet::Come(ComeAttrs {
                        amount,
                        target,
                        odds,
                    }) => {
                        if let Some(t) = target {
                            if *t == sum {
                                let mut winnings = 2 * amount;
                                if let Some(o) = odds {
                                    winnings += odds_payout(*o, *t);
                                }
                                info!("come {t} wins {winnings}");
                                bankroll += winnings;
                            } else {
                                new_bets.push(bet.clone());
                            }
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    bankroll += amount;
                                    info!("come wins on yo");
                                }
                                _ => {
                                    let odds_amount = *amount * odds_multiplier(sum);
                                    let odds = if bankroll >= odds_amount {
                                        bankroll -= odds_amount;
                                        Some(odds_amount)
                                    } else {
                                        None
                                    };
                                    new_bets.push(Bet::Come(ComeAttrs {
                                        amount: *amount,
                                        target: Some(sum),
                                        odds,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        info!("i:{i} point:{point:?} new_point:{new_point:?} bankroll:{bankroll} bets:{bets:?} new_bets:{new_bets:?}");
        bets = new_bets;
        point = new_point;
        if bankroll >= bet_min {
            if point.is_none() {
                bets.push(Bet::Pass(PassAttrs::new(bet_min)));
            } else {
                bets.push(Bet::Come(ComeAttrs::new(bet_min)));
            }
            bankroll -= bet_min;
        }
        info!("bankroll:{bankroll} bets:{bets:?}");
        if bankroll < bet_min && bets.is_empty() {
            break;
        }
    }
    i
}
