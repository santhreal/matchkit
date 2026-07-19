//! concurrent tests for matchkit.
//! See TESTING.md for the Santh testing standard.

use matchkit::{Match, MatchSet};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

#[test]
fn concurrent_matchset_inserts() {
    let set = Arc::new(Mutex::new(MatchSet::new()));
    let num_threads = 10;
    let items_per_thread = 1000;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];

    for i in 0..num_threads {
        let set_clone = Arc::clone(&set);
        let barrier_clone = Arc::clone(&barrier);

        handles.push(thread::spawn(move || {
            barrier_clone.wait(); // Force threads to start simultaneously
            for j in 0..items_per_thread {
                let m = Match::from_parts(
                    (i * 10 + j % 10) as u32,
                    (i * 1000 + j) as u32,
                    (i * 1000 + j + 5) as u32,
                );
                set_clone.lock().unwrap().insert(m);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_set = set.lock().unwrap();
    assert_eq!(final_set.len(), num_threads * items_per_thread);

    // Verify properties of the final set
    let slice = final_set.as_slice();
    for pair in slice.windows(2) {
        assert!(pair[0] <= pair[1]);
    }
}

#[test]
fn concurrent_matchset_extend_and_merge() {
    let set = Arc::new(Mutex::new(MatchSet::new()));
    let num_threads = 8;
    let chunks_per_thread = 50;
    let matches_per_chunk = 1000;

    let mut handles = vec![];

    for i in 0..num_threads {
        let set_clone = Arc::clone(&set);

        handles.push(thread::spawn(move || {
            for c in 0..chunks_per_thread {
                let mut chunk = Vec::with_capacity(matches_per_chunk);
                for j in 0..matches_per_chunk {
                    // Create some overlaps
                    let start = (i * 100_000 + c * 1000 + j * 5) as u32;
                    let end = start + 10;
                    chunk.push(Match::from_parts(0, start, end));
                }

                let mut guard = set_clone.lock().unwrap();
                guard.extend(chunk);
                guard.merge_overlapping();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_set = set.lock().unwrap();
    assert!(!final_set.is_empty(), "Set should contain merged items");

    // Verify properties of the final set
    let slice = final_set.as_slice();
    for pair in slice.windows(2) {
        assert!(pair[0] <= pair[1], "Matches must be sorted");
        assert!(
            !pair[0].overlaps(&pair[1]),
            "No matches should overlap after merge"
        );
    }
}

#[test]
fn concurrent_matchset_32_threads_extreme_hammering() {
    let set = Arc::new(Mutex::new(MatchSet::new()));
    let num_threads = 32;
    let items_per_thread = 5000;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];

    for _ in 0..num_threads {
        let set_clone = Arc::clone(&set);
        let barrier_clone = Arc::clone(&barrier);

        handles.push(thread::spawn(move || {
            barrier_clone.wait(); // Force 32 threads to start simultaneously

            let mut local_batch = Vec::with_capacity(items_per_thread);
            for j in 0..items_per_thread {
                // Highly overlapping and colliding pattern spaces
                let m = Match::from_parts((j % 5) as u32, (j * 2) as u32, (j * 2 + 10) as u32);
                local_batch.push(m);
            }

            // Randomly insert vs extend to cause lock contention and layout shifts
            let mut guard = set_clone.lock().unwrap();
            guard.extend(local_batch);
            guard.merge_overlapping();
        }));
    }

    for handle in handles {
        assert!(
            handle.join().is_ok(),
            "Thread panicked during concurrent hammering"
        );
    }

    let final_set = set.lock().unwrap();
    // After merging overlapping regions of j*2 to j*2+10, it'll become one big chunk for the entire length
    // But per pattern. Since there are 5 patterns, it should be 5 continuous chunks
    assert!(
        !final_set.is_empty(),
        "Set should not be empty after merging"
    );
    assert!(
        final_set.len() <= 5,
        "All continuous matches should merge per pattern"
    );

    let slice = final_set.as_slice();
    for pair in slice.windows(2) {
        assert!(pair[0] <= pair[1], "Final set must remain strictly sorted");
    }
}
