// Copyright 2017, 2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use hex_literal::hex;
use reference_trie::test_layouts;
use trie_db::{
	node::{Node, Value},
	node_db::Hasher,
	NibbleSlice, NibbleVec, TrieDBBuilder, TrieDBNodeIterator, TrieError, TrieIterator, TrieLayout,
};

use crate::TestDB;

pub(crate) fn build_trie_db<T: TrieLayout, DB: TestDB<T>>(
	pairs: &[(Vec<u8>, Vec<u8>)],
) -> (DB, <T::Hash as Hasher>::Out) {
	let mut memdb = DB::default();
	let changeset = {
		let mut t = trie_db::TrieDBMutBuilder::<T>::new(&mut memdb).build();
		for (x, y) in pairs.iter() {
			t.insert(x, y).unwrap();
		}
		t.commit()
	};
	let root = memdb.commit(changeset);
	(memdb, root)
}

pub(crate) fn nibble_vec<T: AsRef<[u8]>>(bytes: T, len: usize) -> NibbleVec {
	let slice = NibbleSlice::new(bytes.as_ref());

	let mut v = NibbleVec::new();
	for i in 0..len {
		v.push(slice.at(i));
	}
	v
}

test_layouts!(iterator_works, iterator_works_internal);
fn iterator_works_internal<T: TrieLayout, DB: TestDB<T>>() {
	let pairs = vec![
		(hex!("01").to_vec(), b"aaaa".to_vec()),
		(hex!("0123").to_vec(), b"bbbb".to_vec()),
		(hex!("02").to_vec(), vec![1; 32]),
	];

	let (memdb, root) = build_trie_db::<T, DB>(&pairs);
	let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

	if T::USE_EXTENSION {
		match iter.next() {
			Some(Ok((prefix, Some(_), node))) => {
				assert_eq!(prefix, nibble_vec(hex!(""), 0));
				match node.node() {
					Node::Extension(partial, _) =>
						assert_eq!(partial, NibbleSlice::new_offset(&hex!("00")[..], 1)),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, Some(_), node))) => {
				assert_eq!(prefix, nibble_vec(hex!("00"), 1));
				match node.node() {
					Node::Branch(..) => {},
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, None, node))) => {
				assert_eq!(prefix, nibble_vec(hex!("01"), 2));
				match node.node() {
					Node::Branch(..) => {},
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, None, node))) => {
				assert_eq!(prefix, nibble_vec(hex!("0120"), 3));
				match node.node() {
					Node::Leaf(partial, ..) =>
						assert_eq!(partial, NibbleSlice::new_offset(&hex!("03")[..], 1)),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, Some(_), node))) => {
				assert_eq!(prefix, nibble_vec(hex!("02"), 2));
				match node.node() {
					Node::Leaf(partial, ..) => assert_eq!(partial, NibbleSlice::new(&hex!("")[..])),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		assert!(iter.next().is_none());
	} else {
		let can_expand =
			T::MAX_INLINE_VALUE.unwrap_or(T::Hash::LENGTH as u32) < T::Hash::LENGTH as u32;
		match iter.next() {
			Some(Ok((prefix, Some(_), node))) => {
				assert_eq!(prefix, nibble_vec(hex!(""), 0));
				match node.node() {
					Node::NibbledBranch(partial, ..) =>
						assert_eq!(partial, NibbleSlice::new_offset(&hex!("00")[..], 1)),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, hash, node))) => {
				if !can_expand {
					assert!(hash.is_none());
				}
				assert_eq!(prefix, nibble_vec(hex!("01"), 2));
				match node.node() {
					Node::NibbledBranch(partial, ..) =>
						assert_eq!(partial, NibbleSlice::new(&hex!("")[..])),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, hash, node))) => {
				if !can_expand {
					assert!(hash.is_none());
				}
				assert_eq!(prefix, nibble_vec(hex!("0120"), 3));
				match node.node() {
					Node::Leaf(partial, ..) =>
						assert_eq!(partial, NibbleSlice::new_offset(&hex!("03")[..], 1)),
					_ => panic!("unexpected node"),
				}
			},

			_ => panic!("unexpected item"),
		}

		match iter.next() {
			Some(Ok((prefix, Some(_), node))) => {
				assert_eq!(prefix, nibble_vec(hex!("02"), 2));
				match node.node() {
					Node::Leaf(partial, ..) => assert_eq!(partial, NibbleSlice::new(&hex!("")[..])),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}

		assert!(iter.next().is_none());
	}
}

test_layouts!(iterator_over_empty_works, iterator_over_empty_works_internal);
fn iterator_over_empty_works_internal<T: TrieLayout, DB: TestDB<T>>() {
	let (memdb, root) = build_trie_db::<T, DB>(&[]);
	let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

	match iter.next() {
		Some(Ok((prefix, Some(_), node))) => {
			assert_eq!(prefix, nibble_vec(hex!(""), 0));
			match node.node() {
				Node::Empty => {},
				_ => panic!("unexpected node"),
			}
		},
		_ => panic!("unexpected item"),
	}

	assert!(iter.next().is_none());
}

test_layouts!(seek_works, seek_works_internal);
fn seek_works_internal<T: TrieLayout, DB: TestDB<T>>() {
	let pairs = vec![
		(hex!("01").to_vec(), b"aaaa".to_vec()),
		(hex!("0123").to_vec(), b"bbbb".to_vec()),
		(hex!("02").to_vec(), vec![1; 32]),
	];

	let (memdb, root) = build_trie_db::<T, DB>(&pairs);
	let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

	TrieIterator::seek(&mut iter, &hex!("")[..]).unwrap();
	match iter.next() {
		Some(Ok((prefix, _, _))) => assert_eq!(prefix, nibble_vec(hex!(""), 0)),
		_ => panic!("unexpected item"),
	}

	TrieIterator::seek(&mut iter, &hex!("00")[..]).unwrap();
	match iter.next() {
		Some(Ok((prefix, _, _))) => assert_eq!(prefix, nibble_vec(hex!("01"), 2)),
		_ => panic!("unexpected item"),
	}

	TrieIterator::seek(&mut iter, &hex!("01")[..]).unwrap();
	match iter.next() {
		Some(Ok((prefix, _, _))) => assert_eq!(prefix, nibble_vec(hex!("01"), 2)),
		_ => panic!("unexpected item"),
	}

	TrieIterator::seek(&mut iter, &hex!("02")[..]).unwrap();
	match iter.next() {
		Some(Ok((prefix, _, _))) => assert_eq!(prefix, nibble_vec(hex!("02"), 2)),
		_ => panic!("unexpected item"),
	}

	TrieIterator::seek(&mut iter, &hex!("03")[..]).unwrap();
	assert!(iter.next().is_none());
}

test_layouts!(seek_over_empty_works, seek_over_empty_works_internal);
fn seek_over_empty_works_internal<T: TrieLayout, DB: TestDB<T>>() {
	let (memdb, root) = build_trie_db::<T, DB>(&[]);
	let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

	TrieIterator::seek(&mut iter, &hex!("")[..]).unwrap();
	match iter.next() {
		Some(Ok((prefix, _, node))) => {
			assert_eq!(prefix, nibble_vec(hex!(""), 0));
			match node.node() {
				Node::Empty => {},
				_ => panic!("unexpected node"),
			}
		},
		_ => panic!("unexpected item"),
	}

	TrieIterator::seek(&mut iter, &hex!("00")[..]).unwrap();
	assert!(iter.next().is_none());
}

test_layouts!(iterate_over_incomplete_db, iterate_over_incomplete_db_internal);
fn iterate_over_incomplete_db_internal<T: TrieLayout, DB: TestDB<T>>()
where
	T::Location: std::fmt::Debug,
{
	let pairs = vec![
		(hex!("01").to_vec(), b"aaaa".to_vec()),
		(hex!("0123").to_vec(), b"bbbb".to_vec()),
		(hex!("02").to_vec(), vec![1; 32]),
		(hex!("03").to_vec(), vec![2; 32]),
	];

	let (mut memdb, root) = build_trie_db::<T, DB>(&pairs);

	// Look up the leaf node with prefix "02".
	let leaf_hash = {
		let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
		let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

		TrieIterator::seek(&mut iter, &hex!("02")[..]).unwrap();
		match iter.next() {
			Some(Ok((_, Some(hash), node))) => match node.node() {
				Node::Leaf(..) => hash,
				_ => panic!("unexpected node"),
			},
			_ => panic!("unexpected item"),
		}
	};

	// Remove the leaf node from the DB.
	let prefix = (&hex!("02")[..], None);
	memdb.remove(&leaf_hash, prefix);

	// Seek to missing node returns error.
	{
		let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
		let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

		match TrieIterator::seek(&mut iter, &hex!("02")[..]) {
			Err(e) =>
				if let TrieError::IncompleteDatabase(err_hash) = *e {
					assert_eq!(err_hash.as_ref(), leaf_hash.as_ref());
				},
			_ => panic!("expected IncompleteDatabase error"),
		}
	}

	// Iterate over missing node works.
	{
		let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
		let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

		TrieIterator::seek(&mut iter, &hex!("0130")[..]).unwrap();
		match iter.next() {
			Some(Err(e)) =>
				if let TrieError::IncompleteDatabase(err_hash) = *e {
					assert_eq!(err_hash.as_ref(), leaf_hash.as_ref());
				},
			_ => panic!("expected IncompleteDatabase error"),
		}
		match iter.next() {
			Some(Ok((_, _, node))) => match node.node() {
				Node::Leaf(_, v, ..) =>
					if !matches!(v, Value::Node(..)) {
						assert_eq!(v, Value::Inline(&vec![2; 32][..]));
					},
				_ => panic!("unexpected node"),
			},
			_ => panic!("unexpected item"),
		}

		assert!(iter.next().is_none());
	}
}

test_layouts!(prefix_works, prefix_works_internal);
fn prefix_works_internal<T: TrieLayout, DB: TestDB<T>>() {
	let can_expand = T::MAX_INLINE_VALUE.unwrap_or(T::Hash::LENGTH as u32) < T::Hash::LENGTH as u32;
	let pairs = vec![
		(hex!("01").to_vec(), b"aaaa".to_vec()),
		(hex!("0123").to_vec(), b"bbbb".to_vec()),
		(hex!("02").to_vec(), vec![1; 32]),
	];

	let (memdb, root) = build_trie_db::<T, DB>(&pairs);
	let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();

	iter.prefix(&hex!("01").to_vec()[..]).unwrap();

	if T::USE_EXTENSION {
		match iter.next() {
			Some(Ok((prefix, None, node))) => {
				assert_eq!(prefix, nibble_vec(hex!("01"), 2));
				match node.node() {
					Node::Branch(..) => {},
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}
	} else {
		match iter.next() {
			Some(Ok((prefix, hash, node))) => {
				if !can_expand {
					debug_assert!(hash.is_none());
				}
				assert_eq!(prefix, nibble_vec(hex!("01"), 2));
				match node.node() {
					Node::NibbledBranch(partial, ..) =>
						assert_eq!(partial, NibbleSlice::new_offset(&hex!("")[..], 0)),
					_ => panic!("unexpected node"),
				}
			},
			_ => panic!("unexpected item"),
		}
	}

	match iter.next() {
		Some(Ok((prefix, hash, node))) => {
			if !can_expand {
				debug_assert!(hash.is_none());
			}
			assert_eq!(prefix, nibble_vec(hex!("0120"), 3));
			match node.node() {
				Node::Leaf(partial, ..) => {
					assert_eq!(partial, NibbleSlice::new_offset(&hex!("03")[..], 1))
				},
				_ => panic!("unexpected node"),
			}
		},
		_ => panic!("unexpected item"),
	}

	assert!(iter.next().is_none());

	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();
	iter.prefix(&hex!("0010").to_vec()[..]).unwrap();
	assert!(iter.next().is_none());
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();
	iter.prefix(&hex!("10").to_vec()[..]).unwrap();
	assert!(iter.next().is_none());
}

test_layouts!(prefix_over_empty_works, prefix_over_empty_works_internal);
fn prefix_over_empty_works_internal<T: TrieLayout, DB: TestDB<T>>() {
	let (memdb, root) = build_trie_db::<T, DB>(&[]);
	let trie = TrieDBBuilder::<T>::new(&memdb, &root).build();
	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();
	iter.prefix(&hex!("")[..]).unwrap();
	match iter.next() {
		Some(Ok((prefix, Some(_), node))) => {
			assert_eq!(prefix, nibble_vec(hex!(""), 0));
			match node.node() {
				Node::Empty => {},
				_ => panic!("unexpected node"),
			}
		},
		_ => panic!("unexpected item"),
	}

	assert!(iter.next().is_none());

	let mut iter = TrieDBNodeIterator::new(&trie).unwrap();
	iter.prefix(&hex!("00")[..]).unwrap();
	assert!(iter.next().is_none());
}
