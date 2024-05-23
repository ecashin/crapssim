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
    #[clap(long, default_value_t = 300)]
    initial_bankroll: usize,
    #[clap(long)]
    n_trials: usize,
    #[clap(long)]
    odds_345: bool,
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

fn odds_multiplier_123(target: usize) -> usize {
    match target {
        4 | 10 => 1,
        5 | 9 => 2,
        6 | 8 => 3,
        _ => panic!("What kind of odds bet was that!? {target}?"),
    }
}

fn odds_multiplier_345(target: usize) -> usize {
    match target {
        4 | 10 => 3,
        5 | 9 => 4,
        6 | 8 => 5,
        _ => panic!("What kind of odds bet was that!? {target}?"),
    }
}

fn main() -> Result<()> {
    simple_logger::init_with_env().context("setting up logging")?;
    let cli = Cli::parse();
    let mut rng = rand::thread_rng();
    let bet_min = 5;
    let mut roll_counts = vec![];
    let mut max_bankrolls = vec![];
    for _ in 1..=cli.n_trials {
        let (n_rolls, max_bankroll) =
            one_scenario(&mut rng, cli.initial_bankroll, bet_min, cli.odds_345);
        roll_counts.push(n_rolls);
        max_bankrolls.push(max_bankroll);
    }
    if cli.n_trials > 1 {
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
        println!("max_bankroll stats:");
        for quantile in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
            let mut a = ndarray::Array1::from_vec(max_bankrolls.clone());
            a.quantile_axis_mut(Axis(0), n64(quantile), &Nearest)
                .with_context(|| format!("computing quantile {quantile}"))?
                .for_each(|v| {
                    let q = format!("q{quantile}");
                    println!("{:>10}: {:>10?}", q, v);
                });
        }
    }
    Ok(())
}

fn increase_bankroll(bankroll: &mut usize, max_bankroll: &mut usize, amount: usize) {
    *bankroll += amount;
    if *bankroll > *max_bankroll {
        *max_bankroll = *bankroll;
    }
}

fn one_scenario(
    rng: &mut ThreadRng,
    initial_bankroll: usize,
    bet_min: usize,
    odds_345: bool,
) -> (usize, usize) {
    let mut bets = vec![Bet::Pass(PassAttrs::new(bet_min))];
    let mut point = None;
    let mut max_bankroll = initial_bankroll;
    let mut bankroll = max_bankroll - bet_min;
    let mut i = 0;
    let odds_multiplier = if odds_345 {
        odds_multiplier_345
    } else {
        odds_multiplier_123
    };
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
                            increase_bankroll(&mut bankroll, &mut max_bankroll, 2 * amount);
                            info!("passline winner");
                        }
                    }
                    Bet::Come(ComeAttrs {
                        amount,
                        target,
                        odds: _,
                    }) => {
                        if target.is_none() {
                            increase_bankroll(&mut bankroll, &mut max_bankroll, 2 * amount);
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
                                increase_bankroll(&mut bankroll, &mut max_bankroll, winnings);
                            } else {
                                new_bets.push(bet.clone());
                            }
                            new_point = point;
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    increase_bankroll(&mut bankroll, &mut max_bankroll, *amount);
                                    info!("pass wins on yo");
                                    new_bets.push(bet.clone())
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
                                increase_bankroll(&mut bankroll, &mut max_bankroll, winnings);
                            } else {
                                new_bets.push(bet.clone());
                            }
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    increase_bankroll(&mut bankroll, &mut max_bankroll, *amount);
                                    info!("come wins on yo");
                                    new_bets.push(bet.clone())
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
            if point.is_none() && !already_pass(&bets) {
                bets.push(Bet::Pass(PassAttrs::new(bet_min)));
                bankroll -= bet_min;
            } else if !already_free_come(&bets) {
                bets.push(Bet::Come(ComeAttrs::new(bet_min)));
                bankroll -= bet_min;
            }
        }
        info!("bankroll:{bankroll} bets:{bets:?}");
        if bankroll < bet_min && bets.is_empty() {
            break;
        }
    }
    (i, max_bankroll)
}

fn already_free_come(bets: &Vec<Bet>) -> bool {
    for bet in bets {
        if matches!(bet, Bet::Come(ComeAttrs { target: None, .. })) {
            return true;
        }
    }
    false
}

fn already_pass(bets: &Vec<Bet>) -> bool {
    for bet in bets {
        if matches!(bet, Bet::Pass(_)) {
            return true;
        }
    }
    false
}
