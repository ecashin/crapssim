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

fn main() {
    let mut rng = rand::thread_rng();
    let bet_min = 5;
    let mut bets = vec![Bet::Pass(PassAttrs::new(bet_min))];
    let mut point = None;
    let mut bankroll = 300 - bet_min;

    for i in 1..=7 {
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
                    Bet::Pass(PassAttrs { amount, odds: _ }) => {
                        if let Some(p) = point {
                            if sum == p {
                                bankroll += 2 * amount;
                                println!("pass wins on point");
                            } else {
                                new_bets.push(bet.clone());
                            }
                            new_point = point;
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    bankroll += 2 * amount;
                                    println!("pass wins on yo");
                                }
                                _ => {
                                    new_point = Some(sum);
                                    new_bets.push(bet.clone());
                                }
                            }
                        }
                    }
                    Bet::Come(ComeAttrs {
                        amount,
                        target,
                        odds: _,
                    }) => {
                        if let Some(t) = target {
                            if *t == sum {
                                bankroll += 2 * amount;
                                println!("come {t} wins");
                            } else {
                                new_bets.push(bet.clone());
                            }
                        } else {
                            match sum {
                                2 | 3 | 12 => (),
                                11 => {
                                    bankroll += 2 * amount;
                                    println!("come wins on yo");
                                }
                                _ => {
                                    new_bets.push(Bet::Come(ComeAttrs {
                                        amount: *amount,
                                        target: Some(sum),
                                        odds: None,
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
        if bankroll > bet_min {
            if point.is_none() {
                bets.push(Bet::Pass(PassAttrs::new(bet_min)));
            } else {
                bets.push(Bet::Come(ComeAttrs::new(bet_min)));
            }
            bankroll -= bet_min;
        }
        println!("bankroll:{bankroll} bets:{bets:?}");
    }
}
