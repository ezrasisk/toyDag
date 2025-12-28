use std::collections::{HashMap, HashSet};
use rand::seq::SliceRandom;

const K: usize = 15; // GHOSTDAG k-parameter (Kaspa uses ~15)
const STITCH_THRESHOLD: usize = 10; // When StitchBot activates

#[derive(Debug, Clone)]
struct Block {
    id: u64,
    parents: Vec<u64>,
    color: Color, // Blue or Red relative to virtual
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Color {
    Blue,
    Red,
}

struct ToyDag {
    blocks: HashMap<u64, Block>,
    tips: HashSet<u64>,
    next_id: u64,
    selected_parent: u64, // Current virtual selected tip
}

impl ToyDag {
    fn new() -> Self {
        let genesis = Block {
            id: 0,
            parents: vec![],
            color: Color::Blue,
        };
        let mut blocks = HashMap::new();
        blocks.insert(0, genesis);

        ToyDag {
            blocks,
            tips: HashSet::from([0]),
            next_id: 1,
            selected_parent: 0,
        }
    }

    // Core GHOSTDAG: compute anticone size relative to selected parent
    fn anticone_size(&self, block_id: u64, reference_id: u64) -> usize {
        // Simplified reachability: count blocks reachable from block but not from reference
        let reachable_from_block = self.future_set(block_id);
        let reachable_from_ref = self.future_set(reference_id);
        reachable_from_block
            .difference(&reachable_from_ref)
            .count()
            - 1 // subtract self
    }

    // Future cone: all blocks that have this as ancestor (including self)
    fn future_set(&self, block_id: u64) -> HashSet<u64> {
        let mut future = HashSet::new();
        let mut queue = vec![block_id];
        future.insert(block_id);

        while let Some(current) = queue.pop() {
            for (&child_id, child) in &self.blocks {
                if child.parents.contains(&current) && future.insert(child_id) {
                    queue.push(child_id);
                }
            }
        }
        future
    }

    // Past cone: all ancestors
    fn past_set(&self, block_id: u64) -> HashSet<u64> {
        let mut past = HashSet::new();
        let mut queue = vec![block_id];
        past.insert(block_id);

        while let Some(current) = queue.pop() {
            for &parent in &self.blocks[&current].parents {
                if past.insert(parent) {
                    queue.push(parent);
                }
            }
        }
        past
    }

    fn create_block(&mut self, parent_ids: Vec<u64>) -> u64 {
        assert!(!parent_ids.is_empty());

        let id = self.next_id;
        self.next_id += 1;

        // Determine color using k-cluster rule
        let color = if self.anticone_size(id, self.selected_parent) <= K {
            Color::Blue
        } else {
            Color::Red
        };

        let block = Block {
            id,
            parents: parent_ids.clone(),
            color,
        };

        self.blocks.insert(id, block);

        // Update tips
        for &pid in &parent_ids {
            if self.tips.len() > 1 || !self.tips.contains(&pid) {
                self.tips.remove(&pid);
            }
        }
        self.tips.insert(id);

        // Update selected parent: heaviest blue tip
        self.update_selected_parent();

        id
    }

    fn update_selected_parent(&mut self) {
        let blue_tips: Vec<u64> = self
            .tips
            .iter()
            .filter(|&&t| self.blocks[&t].color == Color::Blue)
            .copied()
            .collect();

        if let Some(&best) = blue_tips
            .iter()
            .max_by_key(|&&t| self.past_set(t).len()) // heaviest = largest past
        {
            self.selected_parent = best;
        }
    }

    // StitchBot: merge as many tips as possible when too fractured
    fn stitch_if_needed(&mut self) {
        if self.tips.len() > STITCH_THRESHOLD {
            println!("🦸 StitchBot ACTIVATED! Tips: {} → merging all!", self.tips.len());

            let all_tips: Vec<u64> = self.tips.iter().copied().collect();
            let merge_block_id = self.create_block(all_tips.clone());

            println!("🪡 Created merge block {} referencing {} tips", merge_block_id, all_tips.len());
        }
    }

    fn print_dag(&self) {
        println!("=== DAG State ===");
        println!("Blocks: {} | Tips: {} | Selected Parent: {} (color: {:?})",
            self.blocks.len(),
            self.tips.len(),
            self.selected_parent,
            self.blocks[&self.selected_parent].color,
        );

        let mut sorted: Vec<_> = self.blocks.values().collect();
        sorted.sort_by_key(|b| b.id);

        for block in sorted {
            let color_char = match block.color {
                Color::Blue => "🔵",
                Color::Red => "🔴",
            };
            println!(
                "{} Block {} | Parents: {:?} | Past size: {}",
                color_char,
                block.id,
                block.parents,
                self.past_set(block.id).len()
            );
        }
        println!("=================\n");
    }
}

fn main() {
    let mut dag = ToyDag::new();
    let mut rng = rand::thread_rng();

    println!("Starting high-throughput simulation with k={} clustering and StitchBot...\n", K);

    for i in 1..=100 {
        let current_tips: Vec<u64> = dag.tips.iter().copied().collect();
        let num_parents = current_tips.len().min(3); // Up to 3 parents for better merging

        let parents: Vec<u64> = current_tips
            .choose_multiple(&mut rng, num_parents)
            .copied()
            .collect();

        dag.create_block(parents);

        // StitchBot checks every few blocks
        if i % 5 == 0 {
            dag.stitch_if_needed();
        }

        if i % 20 == 0 {
            dag.print_dag();
        }
    }

    println!("Final state: {} blocks, {} tips, selected parent {}",
        dag.blocks.len(), dag.tips.len(), dag.selected_parent);
}
