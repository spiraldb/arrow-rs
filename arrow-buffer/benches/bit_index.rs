// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: See the NOTICE file distributed with this work for additional information regarding copyright ownership
// SPDX-FileContributor: Licensed to the Apache Software Foundation (ASF) under one or more contributor license agreements

use arrow_buffer::BooleanBuffer;
use arrow_buffer::bit_iterator::BitIndexIterator;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::hint::black_box;

fn indices_checksum(values: &[u8], offset: usize, len: usize) -> (usize, usize) {
    BitIndexIterator::new(values, offset, len).fold((0, 0), |(count, sum), index| {
        (count + 1, sum.wrapping_add(index))
    })
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("bit_index");
    let mut rng = StdRng::seed_from_u64(42);

    for len in [31, 1_024, 65_536] {
        let inputs = [
            ("all_zero", vec![false; len]),
            ("all_set", vec![true; len]),
            ("alternating_bits", (0..len).map(|i| i % 2 == 0).collect()),
            (
                "random_half",
                (0..len).map(|_| rng.random_bool(0.5)).collect(),
            ),
            (
                "random_average",
                (0..len).map(|_| rng.random_bool(6.0 / 64.0)).collect(),
            ),
            (
                "random_sparse",
                (0..len).map(|_| rng.random_bool(1.0 / 4_096.0)).collect(),
            ),
            (
                "random_word_density",
                (0..len).map(|_| rng.random_bool(1.0 / 64.0)).collect(),
            ),
            ("one_per_word", (0..len).map(|i| i % 64 == 0).collect()),
            (
                "alternating_words",
                (0..len).map(|i| i % 128 == 0).collect(),
            ),
            ("two_empty_words", (0..len).map(|i| i % 192 == 0).collect()),
            ("six_per_word", (0..len).map(|i| i % 64 < 6).collect()),
            (
                "clustered_middle",
                (0..len)
                    .map(|i| (len / 2..len / 2 + 16).contains(&i))
                    .collect(),
            ),
            (
                "clustered_ends",
                (0..len).map(|i| i < 8 || i >= len - 8).collect(),
            ),
        ];

        // Throughput counts input bits, including those that are not selected.
        group.throughput(Throughput::Elements(len as u64));
        for (name, values) in inputs {
            let expected = values
                .iter()
                .enumerate()
                .filter_map(|(index, selected)| selected.then_some(index))
                .collect::<Vec<_>>();
            let expected_checksum = expected.iter().fold((0, 0usize), |(count, sum), &index| {
                (count + 1, sum.wrapping_add(index))
            });
            for offset in [0, 61] {
                let buffer = BooleanBuffer::from_iter(
                    std::iter::repeat_n(true, offset)
                        .chain(values.iter().copied())
                        .chain([true; 7]),
                );
                assert_eq!(
                    BitIndexIterator::new(buffer.values(), offset, len).collect::<Vec<_>>(),
                    expected
                );
                assert_eq!(
                    indices_checksum(buffer.values(), offset, len),
                    expected_checksum
                );
                let id = BenchmarkId::new(format!("{name}/offset_{offset}"), len);
                group.bench_with_input(id, &buffer, |b, buffer| {
                    b.iter(|| {
                        black_box(indices_checksum(
                            black_box(buffer.values()),
                            black_box(offset),
                            black_box(len),
                        ))
                    });
                });
            }
        }
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
