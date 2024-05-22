use std::fmt;

use rand::{rngs::ThreadRng, Rng};

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

fn main() {
    let mut rng = rand::thread_rng();
    let bet_min = 5;
    let mut bets = vec![Bet::Pass(PassAttrs::new(bet_min))];
    let mut point = None;
    let mut bankroll = 300 - bet_min;

    for i in 1.. {
        let mut new_bets = vec![];
        let dice = roll(&mut rng);
        let sum = dice.0 + dice.1;
        println!("i:{i} roll:{dice:?} sum:{sum}");
        let mut new_point: Option<usize> = None;
        if sum == 7 {
            for bet in &bets {
                match bet {
                    Bet::Pass(PassAttrs { amount, odds: _ }) => {
                        if point.is_none() {
                            bankroll += 2 * amount;
                            println!("passline winner");
                        }
                    }
                    Bet::Come(ComeAttrs {
                        amount,
                        target,
                        odds: _,
                    }) => {
                        if target.is_none() {
                            bankroll += 2 * amount;
                            println!("come wins");
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
                                println!("pass wins {winnings} on point");
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
                                    println!("pass wins on yo");
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
                                println!("come {t} wins {winnings}");
                                bankroll += winnings;
                            } else {
                                new_bets.push(bet.clone());
                            }
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    bankroll += amount;
                                    println!("come wins on yo");
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
        println!("i:{i} point:{point:?} new_point:{new_point:?} bankroll:{bankroll} bets:{bets:?} new_bets:{new_bets:?}");
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
        println!("bankroll:{bankroll} bets:{bets:?}");
        if bankroll < bet_min && bets.is_empty() {
            break;
        }
    }
}
