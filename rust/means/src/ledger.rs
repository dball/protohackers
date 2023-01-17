use std::collections::BTreeMap;

pub struct Ledger {
  entries: BTreeMap<Inst, Price>
}

type Price = i32;
type Inst = i32;

impl Ledger {
  pub fn new() -> Self {
    Self { entries: BTreeMap::new() }
  }

  pub fn insert(&mut self, inst: Inst, price: Price) -> bool {
    if self.entries.contains_key(&inst) {
      return false;
    }
    self.entries.insert(inst, price);
    return true;
  }

  pub fn mean(&self, min: Inst, max:Inst) -> Option<Price> {
    if min > max {
      return None
    }
    let mut n = 0;
    let mut sum: f64 = 0.0;
    for (_, price) in self.entries.range(min..max+1) {
      n += 1;
      sum += *price as f64;
    }
    if n == 0 {
      return None
    } else {
      return Some((sum / n as f64).round() as Price)
    }
  }
}