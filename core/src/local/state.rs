use ::*;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use local::keyed_state::KeyedState;

struct SingleState<T: Hash + Eq + Clone + 'static> {
    key: Vec<usize>,
    state: KeyedState<T>,
    partial: bool,
}

pub struct State<T: Hash + Eq + Clone + 'static> {
    state: Vec<SingleState<T>>,
    by_tag: HashMap<Tag, usize>,
    rows: usize,
}

impl<T: Hash + Eq + Clone + 'static> Default for State<T> {
    fn default() -> Self {
        State {
            state: Vec::new(),
            by_tag: HashMap::new(),
            rows: 0,
        }
    }
}

impl<T: Hash + Eq + Clone + 'static> State<T> {
    /// Construct base materializations differently (potentially)
    pub fn base() -> Self {
        Self::default()
    }

    fn state_for(&self, cols: &[usize]) -> Option<usize> {
        self.state.iter().position(|s| &s.key[..] == cols)
    }

    pub fn add_key(&mut self, columns: &[usize], partial: Option<Vec<Tag>>) {
        let (i, exists) = if let Some(i) = self.state_for(columns) {
            // already keyed by this key; just adding tags
            (i, true)
        } else {
            // will eventually be assigned
            (self.state.len(), false)
        };

        let is_partial = partial.is_some();
        if let Some(ref p) = partial {
            for &tag in p {
                self.by_tag.insert(tag, i);
            }
        }

        if exists {
            return;
        }

        self.state.push(SingleState {
            key: Vec::from(columns),
            state: columns.into(),
            partial: is_partial,
        });

        if !self.is_empty() {
            // we need to *construct* the index!
            if is_partial {
                // partial views can start out empty
                return;
            }

            let (new, old) = self.state.split_last_mut().unwrap();
            let mut insert = move |rs: &Vec<Row<Vec<T>>>| {
                for r in rs {
                    State::insert_into(new, Row(r.0.clone()));
                }
            };
            match old[0].state {
                KeyedState::Single(ref map) => for rs in map.values() {
                    insert(rs);
                },
                KeyedState::Double(ref map) => for rs in map.values() {
                    insert(rs);
                },
                KeyedState::Tri(ref map) => for rs in map.values() {
                    insert(rs);
                },
                KeyedState::Quad(ref map) => for rs in map.values() {
                    insert(rs);
                },
                KeyedState::Quin(ref map) => for rs in map.values() {
                    insert(rs);
                },
                KeyedState::Sex(ref map) => for rs in map.values() {
                    insert(rs);
                },
            }
        }
    }

    pub fn keys(&self) -> Vec<Vec<usize>> {
        self.state.iter().map(|s| &s.key).cloned().collect()
    }

    pub fn is_useful(&self) -> bool {
        !self.state.is_empty()
    }

    pub fn is_partial(&self) -> bool {
        self.state.iter().any(|s| s.partial)
    }

    /// Insert the given record into the given state.
    ///
    /// Returns false if a hole was encountered (and the record hence not inserted).
    fn insert_into(s: &mut SingleState<T>, r: Row<Vec<T>>) -> bool {
        use rahashmap::Entry;
        match s.state {
            KeyedState::Single(ref mut map) => {
                // treat this specially to avoid the extra Vec
                debug_assert_eq!(s.key.len(), 1);
                // i *wish* we could use the entry API here, but it would mean an extra clone
                // in the common case of an entry already existing for the given key...
                if let Some(ref mut rs) = map.get_mut(&r[s.key[0]]) {
                    rs.push(r);
                    return true;
                } else if s.partial {
                    // trying to insert a record into partial materialization hole!
                    return false;
                }
                map.insert(r[s.key[0]].clone(), vec![r]);
            }
            KeyedState::Double(ref mut map) => {
                let key = (r[s.key[0]].clone(), r[s.key[1]].clone());
                match map.entry(key) {
                    Entry::Occupied(mut rs) => rs.get_mut().push(r),
                    Entry::Vacant(..) if s.partial => return false,
                    rs @ Entry::Vacant(..) => rs.or_default().push(r),
                }
            }
            KeyedState::Tri(ref mut map) => {
                let key = (
                    r[s.key[0]].clone(),
                    r[s.key[1]].clone(),
                    r[s.key[2]].clone(),
                );
                match map.entry(key) {
                    Entry::Occupied(mut rs) => rs.get_mut().push(r),
                    Entry::Vacant(..) if s.partial => return false,
                    rs @ Entry::Vacant(..) => rs.or_default().push(r),
                }
            }
            KeyedState::Quad(ref mut map) => {
                let key = (
                    r[s.key[0]].clone(),
                    r[s.key[1]].clone(),
                    r[s.key[2]].clone(),
                    r[s.key[3]].clone(),
                );
                match map.entry(key) {
                    Entry::Occupied(mut rs) => rs.get_mut().push(r),
                    Entry::Vacant(..) if s.partial => return false,
                    rs @ Entry::Vacant(..) => rs.or_default().push(r),
                }
            }
            KeyedState::Quin(ref mut map) => {
                let key = (
                    r[s.key[0]].clone(),
                    r[s.key[1]].clone(),
                    r[s.key[2]].clone(),
                    r[s.key[3]].clone(),
                    r[s.key[4]].clone(),
                );
                match map.entry(key) {
                    Entry::Occupied(mut rs) => rs.get_mut().push(r),
                    Entry::Vacant(..) if s.partial => return false,
                    rs @ Entry::Vacant(..) => rs.or_default().push(r),
                }
            }
            KeyedState::Sex(ref mut map) => {
                let key = (
                    r[s.key[0]].clone(),
                    r[s.key[1]].clone(),
                    r[s.key[2]].clone(),
                    r[s.key[3]].clone(),
                    r[s.key[4]].clone(),
                    r[s.key[5]].clone(),
                );
                match map.entry(key) {
                    Entry::Occupied(mut rs) => rs.get_mut().push(r),
                    Entry::Vacant(..) if s.partial => return false,
                    rs @ Entry::Vacant(..) => rs.or_default().push(r),
                }
            }
        }

        true
    }

    pub fn insert(&mut self, r: Vec<T>, partial_tag: Option<Tag>) -> bool {
        let r = Rc::new(r);

        if let Some(tag) = partial_tag {
            let i = match self.by_tag.get(&tag) {
                Some(i) => *i,
                None => {
                    // got tagged insert for unknown tag. this will happen if a node on an old
                    // replay path is now materialized. must return true to avoid any records
                    // (which are destined for a downstream materialization) from being pruned.
                    return true;
                }
            };
            // FIXME: self.rows += ?
            State::insert_into(&mut self.state[i], Row(r))
        } else {
            let mut hit_any = true;
            self.rows = self.rows.saturating_add(1);
            for i in 0..self.state.len() {
                hit_any = State::insert_into(&mut self.state[i], Row(r.clone())) || hit_any;
            }
            hit_any
        }
    }

    pub fn remove(&mut self, r: &[T]) -> bool {
        let mut hit = false;
        let mut removed = false;
        let fix = |removed: &mut bool, rs: &mut Vec<Row<Vec<T>>>| {
            // rustfmt
            if let Some(i) = rs.iter().position(|rsr| &rsr[..] == r) {
                rs.swap_remove(i);
                *removed = true;
            }
        };

        for s in &mut self.state {
            match s.state {
                KeyedState::Single(ref mut map) => {
                    if let Some(ref mut rs) = map.get_mut(&r[s.key[0]]) {
                        fix(&mut removed, rs);
                        hit = true;
                    }
                }
                KeyedState::Double(ref mut map) => {
                    // TODO: can we avoid the Clone here?
                    let key = (r[s.key[0]].clone(), r[s.key[1]].clone());
                    if let Some(ref mut rs) = map.get_mut(&key) {
                        fix(&mut removed, rs);
                        hit = true;
                    }
                }
                KeyedState::Tri(ref mut map) => {
                    let key = (
                        r[s.key[0]].clone(),
                        r[s.key[1]].clone(),
                        r[s.key[2]].clone(),
                    );
                    if let Some(ref mut rs) = map.get_mut(&key) {
                        fix(&mut removed, rs);
                        hit = true;
                    }
                }
                KeyedState::Quad(ref mut map) => {
                    let key = (
                        r[s.key[0]].clone(),
                        r[s.key[1]].clone(),
                        r[s.key[2]].clone(),
                        r[s.key[3]].clone(),
                    );
                    if let Some(ref mut rs) = map.get_mut(&key) {
                        fix(&mut removed, rs);
                        hit = true;
                    }
                }
                KeyedState::Quin(ref mut map) => {
                    let key = (
                        r[s.key[0]].clone(),
                        r[s.key[1]].clone(),
                        r[s.key[2]].clone(),
                        r[s.key[3]].clone(),
                        r[s.key[4]].clone(),
                    );
                    if let Some(ref mut rs) = map.get_mut(&key) {
                        fix(&mut removed, rs);
                        hit = true;
                    }
                }
                KeyedState::Sex(ref mut map) => {
                    let key = (
                        r[s.key[0]].clone(),
                        r[s.key[1]].clone(),
                        r[s.key[2]].clone(),
                        r[s.key[3]].clone(),
                        r[s.key[4]].clone(),
                        r[s.key[5]].clone(),
                    );
                    if let Some(ref mut rs) = map.get_mut(&key) {
                        fix(&mut removed, rs);
                        hit = true;
                    }
                }
            }
        }

        if removed {
            self.rows = self.rows.saturating_sub(1);
        }

        hit
    }

    pub fn iter(&self) -> rahashmap::Values<T, Vec<Row<Vec<T>>>> {
        for index in &self.state {
            if let KeyedState::Single(ref map) = index.state {
                if index.partial {
                    unimplemented!();
                }
                return map.values();
            }
        }
        // TODO: allow iter without single key (breaks return type)
        unimplemented!();
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty() || self.state[0].state.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows
    }

    pub fn nkeys(&self) -> usize {
        if self.state.is_empty() {
            0
        } else {
            self.state[0].state.len()
        }
    }

    pub fn mark_filled(&mut self, key: Vec<T>, tag: &Tag) {
        debug_assert!(!self.state.is_empty(), "filling uninitialized index");
        let i = self.by_tag[tag];
        let index = &mut self.state[i];
        let mut key = key.into_iter();
        let replaced = match index.state {
            KeyedState::Single(ref mut map) => map.insert(key.next().unwrap(), Vec::new()),
            KeyedState::Double(ref mut map) => {
                map.insert((key.next().unwrap(), key.next().unwrap()), Vec::new())
            }
            KeyedState::Tri(ref mut map) => map.insert(
                (
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                ),
                Vec::new(),
            ),
            KeyedState::Quad(ref mut map) => map.insert(
                (
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                ),
                Vec::new(),
            ),
            KeyedState::Quin(ref mut map) => map.insert(
                (
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                ),
                Vec::new(),
            ),
            KeyedState::Sex(ref mut map) => map.insert(
                (
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                    key.next().unwrap(),
                ),
                Vec::new(),
            ),
        };
        assert!(replaced.is_none());
    }

    pub fn mark_hole(&mut self, key: &[T], tag: &Tag) {
        debug_assert!(!self.state.is_empty(), "filling uninitialized index");
        let i = self.by_tag[tag];
        let index = &mut self.state[i];
        let removed = match index.state {
            KeyedState::Single(ref mut map) => map.remove(&key[0]),
            KeyedState::Double(ref mut map) => map.remove(&(key[0].clone(), key[1].clone())),
            KeyedState::Tri(ref mut map) => {
                map.remove(&(key[0].clone(), key[1].clone(), key[2].clone()))
            }
            KeyedState::Quad(ref mut map) => map.remove(&(
                key[0].clone(),
                key[1].clone(),
                key[2].clone(),
                key[3].clone(),
            )),
            KeyedState::Quin(ref mut map) => map.remove(&(
                key[0].clone(),
                key[1].clone(),
                key[2].clone(),
                key[3].clone(),
                key[4].clone(),
            )),
            KeyedState::Sex(ref mut map) => map.remove(&(
                key[0].clone(),
                key[1].clone(),
                key[2].clone(),
                key[3].clone(),
                key[4].clone(),
                key[5].clone(),
            )),
        };
        // mark_hole should only be called on keys we called mark_filled on
        assert!(removed.is_some());
    }

    pub fn lookup<'a>(&'a self, columns: &[usize], key: &KeyType<T>) -> LookupResult<'a, T> {
        debug_assert!(!self.state.is_empty(), "lookup on uninitialized index");
        let index = &self.state[self.state_for(columns)
                                    .expect("lookup on non-indexed column set")];
        if let Some(rs) = index.state.lookup(key) {
            LookupResult::Some(&rs[..])
        } else {
            if index.partial {
                // partially materialized, so this is a hole (empty results would be vec![])
                LookupResult::Missing
            } else {
                LookupResult::Some(&[])
            }
        }
    }

    fn fix<'a>(rs: &'a Vec<Row<Vec<T>>>) -> impl Iterator<Item = Vec<T>> + 'a {
        rs.iter().map(|r| Vec::clone(&**r))
    }

    pub fn cloned_records(&self) -> Vec<Vec<T>> {
        match self.state[0].state {
            KeyedState::Single(ref map) => map.values().flat_map(State::fix).collect(),
            KeyedState::Double(ref map) => map.values().flat_map(State::fix).collect(),
            KeyedState::Tri(ref map) => map.values().flat_map(State::fix).collect(),
            KeyedState::Quad(ref map) => map.values().flat_map(State::fix).collect(),
            KeyedState::Quin(ref map) => map.values().flat_map(State::fix).collect(),
            KeyedState::Sex(ref map) => map.values().flat_map(State::fix).collect(),
        }
    }

    pub fn clear(&mut self) {
        self.rows = 0;
        for s in &mut self.state {
            match s.state {
                KeyedState::Single(ref mut map) => map.clear(),
                KeyedState::Double(ref mut map) => map.clear(),
                KeyedState::Tri(ref mut map) => map.clear(),
                KeyedState::Quad(ref mut map) => map.clear(),
                KeyedState::Quin(ref mut map) => map.clear(),
                KeyedState::Sex(ref mut map) => map.clear(),
            }
        }
    }
}

impl<'a, T: Eq + Hash + Clone + 'static> State<T> {
    fn unalias_for_state(&mut self) {
        let left = self.state.drain(..).last();
        if let Some(left) = left {
            self.state.push(left);
        }
    }
}

impl<'a, T: Eq + Hash + Clone + 'static> Drop for State<T> {
    fn drop(&mut self) {
        self.unalias_for_state();
        self.clear();
    }
}

impl<T: Hash + Eq + Clone + 'static> IntoIterator for State<T> {
    type Item = Vec<Vec<T>>;
    type IntoIter = Box<Iterator<Item = Self::Item>>;
    fn into_iter(mut self) -> Self::IntoIter {
        // we need to make sure that the records eventually get dropped, so we need to ensure there
        // is only one index left (which therefore owns the records), and then cast back to the
        // original boxes.
        self.unalias_for_state();
        let own = |rs: Vec<Row<Vec<T>>>| match rs.into_iter()
            .map(|r| Rc::try_unwrap(r.0))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(rs) => rs,
            Err(_) => unreachable!("rc still not owned after unaliasing"),
        };
        self.state
            .drain(..)
            .last()
            .map(move |index| -> Self::IntoIter {
                match index.state {
                    KeyedState::Single(map) => Box::new(map.into_iter().map(move |(_, v)| own(v))),
                    KeyedState::Double(map) => Box::new(map.into_iter().map(move |(_, v)| own(v))),
                    KeyedState::Tri(map) => Box::new(map.into_iter().map(move |(_, v)| own(v))),
                    KeyedState::Quad(map) => Box::new(map.into_iter().map(move |(_, v)| own(v))),
                    KeyedState::Quin(map) => Box::new(map.into_iter().map(move |(_, v)| own(v))),
                    KeyedState::Sex(map) => Box::new(map.into_iter().map(move |(_, v)| own(v))),
                }
            })
            .unwrap()
    }
}