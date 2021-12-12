use super::BoxedStageLabel;
use bevy_utils::{HashMap, HashSet};
use std::collections::LinkedList;

#[derive(Default)]
pub(crate) struct StageOrder {
    edges: HashMap<BoxedStageLabel, HashSet<BoxedStageLabel>>,
    incoming_count: HashMap<BoxedStageLabel, usize>,
}

impl StageOrder {
    pub fn add_stage(&mut self, label: BoxedStageLabel) {
        if self.edges.contains_key(&label) {
            panic!("Stage already exists: {:?}.", label);
        } else {
            self.edges.insert(label.clone(), HashSet::default());
            self.incoming_count.insert(label, 0);
        }
    }

    pub fn add_stage_after(&mut self, target: BoxedStageLabel, label: BoxedStageLabel) {
        self.add_stage(label.clone());

        let edges_entry = self
            .edges
            .get_mut(&target)
            .unwrap_or_else(|| panic!("Target stage should exist: {:?}", target));

        if !edges_entry.contains(&label) {
            let incoming_counter = self
                .incoming_count
                .get_mut(&label)
                .expect("Label counter should be created");
            *incoming_counter += 1;

            edges_entry.insert(label.clone());
        }
    }

    pub fn add_stage_before(&mut self, target: BoxedStageLabel, label: BoxedStageLabel) {
        self.add_stage(label.clone());

        let edges_entry = self
            .edges
            .get_mut(&label)
            .unwrap_or_else(|| panic!("Label stage should exist: {:?}", label));

        if !edges_entry.contains(&target) {
            let incoming_counter = self.incoming_count.get_mut(&target).unwrap_or_else(|| {
                panic!("Target counter should be created: {:?}", target.clone())
            });
            *incoming_counter += 1;

            edges_entry.insert(target.clone());
        }
    }

    pub fn iter(&self) -> StageOrderIterator {
        StageOrderIterator::from_stage_order(self)
    }
}

pub(crate) struct StageOrderIterator<'a> {
    incoming_count: HashMap<BoxedStageLabel, usize>,
    queue: LinkedList<BoxedStageLabel>,
    stage_order: &'a StageOrder,
}

impl<'a> StageOrderIterator<'a> {
    fn from_stage_order(stage_order: &'a StageOrder) -> Self {
        let incoming_count = stage_order.incoming_count.clone();
        let queue = incoming_count
            .iter()
            .filter(|(_, value)| **value == 0)
            .map(|(key, _)| key.clone())
            .collect();

        Self {
            queue,
            stage_order,
            incoming_count,
        }
    }
}

impl<'a> Iterator for StageOrderIterator<'a> {
    type Item = BoxedStageLabel;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(label) = self.queue.pop_back() {
            let neighbours = self
                .stage_order
                .edges
                .get(&label)
                .expect("Edges should be present");

            for neighbour in neighbours {
                let counter = self.incoming_count.get_mut(neighbour).unwrap_or_else(|| {
                    panic!("Neighbour counter should be present: {:?}", neighbour)
                });
                *counter -= 1;

                if *counter == 0 {
                    self.queue.push_front(neighbour.clone());
                }
            }

            Some(label)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_utils::HashMap;

    use crate::schedule::{BoxedStageLabel, StageLabel};

    use super::StageOrder;

    #[derive(Debug, Hash, PartialEq, Eq, Clone)]
    enum Labels {
        First,
        Second,
        Third,
        Forth,
        Other,
    }

    impl StageLabel for Labels {
        fn dyn_clone(&self) -> Box<dyn StageLabel> {
            Box::new(Clone::clone(self))
        }
    }

    #[test]
    fn test_preserve_order() {
        let mut stage_order = StageOrder::default();

        // Add stages in the order
        // F - S - T - F
        //       \
        //         O
        stage_order.add_stage(Labels::Second.dyn_clone());
        stage_order.add_stage_before(Labels::Second.dyn_clone(), Labels::First.dyn_clone());

        stage_order.add_stage_after(Labels::Second.dyn_clone(), Labels::Other.dyn_clone());

        stage_order.add_stage_after(Labels::Second.dyn_clone(), Labels::Third.dyn_clone());
        stage_order.add_stage_after(Labels::Third.dyn_clone(), Labels::Forth.dyn_clone());

        // Build order hashmap. Each value is index in the returned iterator
        let order: HashMap<BoxedStageLabel, usize> = stage_order
            .iter()
            .into_iter()
            .enumerate()
            .map(|(idx, label)| (label.clone(), idx))
            .collect();

        // Verify that order is preserved
        assert_eq!(order.len(), 5, "Orders length should be 5");
        assert!(order[&Labels::First.dyn_clone()] < order[&Labels::Second.dyn_clone()]);
        assert!(order[&Labels::Second.dyn_clone()] < order[&Labels::Third.dyn_clone()]);
        assert!(order[&Labels::Third.dyn_clone()] < order[&Labels::Forth.dyn_clone()]);
        assert!(order[&Labels::Second.dyn_clone()] < order[&Labels::Other.dyn_clone()]);
    }

    #[test]
    #[should_panic]
    fn test_adding_same_stage_twice() {
        let mut stage_order = StageOrder::default();

        stage_order.add_stage(Labels::Other.dyn_clone());
        stage_order.add_stage(Labels::Other.dyn_clone());
    }

    #[test]
    #[should_panic]
    fn test_adding_with_missing_target() {
        let mut stage_order = StageOrder::default();

        stage_order.add_stage_before(Labels::Second.dyn_clone(), Labels::First.dyn_clone());
    }
}
