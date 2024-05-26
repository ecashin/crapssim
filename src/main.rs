use std::{
    fmt,
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use ndarray::Axis;
use ndarray_stats::{interpolate::Nearest, QuantileExt};
use noisy_float::types::n64;
use rand::{thread_rng, Rng};

#[derive(Parser)]
struct Cli {
    #[clap(long, default_value_t = 300)]
    initial_bankroll: usize,
    #[clap(long)]
    n_trials: usize,
    #[clap(long, default_value = "123")]
    odds: String,
    #[clap(long)]
    roll_script: Option<PathBuf>,
    #[clap(long)]
    roll_log: Option<PathBuf>,
    #[clap(long)]
    grow_bets: bool,
    #[clap(long)]
    grow_odds: bool,
}

type Roll = (usize, usize);

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
        6 | 8 => (6, 5),
        _ => panic!("What kind of odds bet was that!? {target}?"),
    };
    (amount * numerator) / denominator
}

fn odds_multiplier_10(_target: usize) -> usize {
    10
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
    let bet_min = 5;
    let mut roll_counts = vec![];
    let mut max_bankrolls = vec![];
    for _ in 1..=cli.n_trials {
        let (n_rolls, max_bankroll) = one_scenario(
            cli.initial_bankroll,
            bet_min,
            &cli.odds,
            &cli.roll_script,
            &cli.roll_log,
            cli.grow_bets,
            cli.grow_odds,
        );
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

struct Shooter {
    values: Option<Vec<Roll>>,
    i: usize,
    roll_logger: RollLogger,
}

impl Shooter {
    fn new(script: &Option<PathBuf>, roll_log: &Option<PathBuf>) -> Self {
        let values = if let Some(script) = script {
            let rolls: Vec<Roll> = fs::read_to_string(script)
                .unwrap()
                .lines()
                .map(|line| {
                    let mut fields = line.split_whitespace();
                    (
                        fields.next().unwrap().parse().unwrap(),
                        fields.next().unwrap().parse().unwrap(),
                    )
                })
                .collect();
            Some(rolls)
        } else {
            None
        };
        let i = 0;
        let roll_logger = RollLogger::new(roll_log);
        Self {
            values,
            i,
            roll_logger,
        }
    }

    fn roll(&mut self) -> Roll {
        let roll = if let Some(values) = &self.values {
            let roll = values[self.i % values.len()];
            self.i += 1;
            roll
        } else {
            (thread_rng().gen_range(1..=6), thread_rng().gen_range(1..=6))
        };
        self.roll_logger.log(roll);
        roll
    }
}

struct RollLogger {
    writer: Option<BufWriter<File>>,
}

impl RollLogger {
    fn new(roll_log: &Option<PathBuf>) -> Self {
        let writer = if let Some(log_file) = roll_log {
            let file = File::create(log_file).expect("opening roll log");
            Some(BufWriter::new(file))
        } else {
            None
        };
        Self { writer }
    }

    fn log(&mut self, roll: Roll) {
        if let Some(writer) = &mut self.writer {
            writeln!(writer, "{} {}", roll.0, roll.1).expect("writing roll to log");
        }
    }
}

fn grow_odds_multiplier(target: usize, initial_bankroll: usize, bankroll: usize) -> usize {
    let initial = initial_bankroll as f64;
    let current = bankroll as f64;
    let f = current / initial;
    if f < 0.8 {
        odds_multiplier_123(target)
    } else if f < 1.4 {
        odds_multiplier_345(target)
    } else {
        odds_multiplier_10(target)
    }
}

fn one_scenario(
    initial_bankroll: usize,
    bet_min: usize,
    odds: &str,
    roll_script: &Option<PathBuf>,
    roll_log: &Option<PathBuf>,
    grow_bets: bool,
    grow_odds: bool,
) -> (usize, usize) {
    let mut bets = vec![Bet::Pass(PassAttrs::new(bet_min))];
    let mut point = None;
    let mut max_bankroll = initial_bankroll;
    let mut bankroll = max_bankroll - bet_min;
    let mut shooter = Shooter::new(roll_script, roll_log);
    let mut i = 0;
    let odds_multiplier = |target, initial_bankroll, bankroll| {
        if grow_odds {
            grow_odds_multiplier(target, initial_bankroll, bankroll)
        } else {
            match odds {
                "345" => odds_multiplier_345(target),
                "123" => odds_multiplier_123(target),
                "10" => odds_multiplier_10(target),
                _ => panic!("not an odds type"),
            }
        }
    };
    loop {
        i += 1;
        let mut new_bets = vec![];
        let dice = shooter.roll();
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
                    // non-7 pass bet
                    Bet::Pass(PassAttrs { amount, odds }) => {
                        if let Some(p) = point {
                            if sum == p {
                                let mut winnings = 2 * amount;
                                if let Some(o) = odds {
                                    winnings += *o;
                                    winnings += odds_payout(*o, p);
                                }
                                info!("pass wins {winnings} on point");
                                increase_bankroll(&mut bankroll, &mut max_bankroll, winnings);
                            } else {
                                new_bets.push(bet.clone());
                                new_point = point;
                            }
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
                                    let odds_amount =
                                        *amount * odds_multiplier(sum, initial_bankroll, bankroll);
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
                    // non-7 come bet
                    Bet::Come(ComeAttrs {
                        amount,
                        target,
                        odds,
                    }) => {
                        if let Some(t) = target {
                            if *t == sum {
                                let mut winnings = 2 * amount;
                                if let Some(o) = odds {
                                    winnings += *o;
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
                                    let odds_amount =
                                        *amount * odds_multiplier(sum, initial_bankroll, bankroll);
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
            let mut bet_amount = bet_min;
            let mut big = bankroll;
            if grow_bets {
                loop {
                    big /= 2;
                    if big < initial_bankroll {
                        break;
                    }
                    bet_amount *= 2;
                }
            }
            if point.is_none() && !already_pass(&bets) {
                bets.push(Bet::Pass(PassAttrs::new(bet_amount)));
                bankroll -= bet_amount;
            } else if !already_free_come(&bets) {
                bets.push(Bet::Come(ComeAttrs::new(bet_amount)));
                bankroll -= bet_amount;
            }
        }
        info!("bankroll:{bankroll} bets:{bets:?}");
        if bankroll < bet_min && bets.is_empty() {
            break;
        }
    }
    info!("i{i}: max_bankroll:{max_bankroll}");
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
