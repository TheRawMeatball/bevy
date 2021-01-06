use std::collections::BTreeMap;

use bevy_ecs::{Entity, Flags, Query};
use bevy_math::Vec2;
use bevy_text::CalculatedSize;
use bevy_transform::components::{Children, Transform};

use crate::Node;

use super::*;

pub(crate) fn solve(
    solve_entity: Entity,
    space: Vec2,
    respect_flags: bool,
    nodes: &Query<(
        &AnchorLayout,
        Flags<AnchorLayout>,
        Option<&CalculatedSize>,
        Option<&Children>,
        Option<Flags<Children>>,
    )>,
    mutables: &mut Query<(&mut Transform, &mut Node, &mut ANodeLayoutCache), With<AnchorLayout>>,
) {
    let (mut target_transform, mut render_data, cache) = mutables.get_mut(solve_entity).unwrap();
    let target_size = &mut render_data.size;
    let (solve_target, node_flags, c_size, children, children_flags) =
        nodes.get(solve_entity).unwrap();

    if respect_flags && !node_flags.changed() && !c_size.map(|f| f.dirty).unwrap_or(false) {
        if let Some(children) = children {
            let solve_self =
                |transforms| solve(solve_entity, space, false, nodes, transforms);
            let ts = *target_size;
            if solve_target.children_spread.is_some() {
                if children_flags.unwrap().changed() {
                    solve_self(mutables);
                    return;
                }
                for child in children.iter() {
                    let child = nodes.get(*child).unwrap();
                    if child.1.changed() || child.2.map(|cs| cs.dirty).unwrap_or(false) {
                        solve_self(mutables);
                        return;
                    }
                }
                let cache = cache.sizes.as_ref().unwrap().clone();
                for (child, size) in children.iter().zip(cache.iter()) {
                    solve(*child, *size, true, nodes, mutables)
                }
            } else {
                for child in children.iter() {
                    solve(*child, ts, true, nodes, mutables)
                }
            }
        }
        return;
    }

    let csize = (c_size.map(|c| c.size.width), c_size.map(|c| c.size.height));
    let mut offset = match &solve_target.constraint {
        Constraint::Independent { x, y } => {
            let x = x.solve(solve_target.anchors.x(), space.x, csize.0);
            let y = y.solve(solve_target.anchors.y(), space.y, csize.1);

            *target_size = Vec2::new(x.size, y.size);
            Vec2::new(x.offset, y.offset)
        }
        Constraint::SetXWithY { x, y, aspect } => {
            let y = y.solve(solve_target.anchors.y(), space.y, csize.1);
            let aspect = aspect.unwrap_or_else(|| {
                c_size
                    .map(|cs| cs.size.width / cs.size.height)
                    .unwrap_or(1.)
            });
            let x = match x {
                Alignment::DirectMargin(m) => AxisConstraint::DirectMarginAndSize(
                    *m,
                    y.size * aspect,
                )
                .solve(solve_target.anchors.x(), space.x, None),
                Alignment::ReverseMargin(m) => AxisConstraint::ReverseMarginAndSize(
                    *m,
                    y.size / aspect,
                )
                .solve(solve_target.anchors.x(), space.x, None),
                Alignment::Offset(o) => AxisConstraintSolve {
                    offset: *o,
                    size: y.size / aspect,
                },
            };

            *target_size = Vec2::new(x.size, y.size);
            Vec2::new(x.offset, y.offset)
        }
        Constraint::SetYWithX { x, y, aspect } => {
            let x = x.solve(solve_target.anchors.x(), space.x, csize.0);
            let aspect = aspect.unwrap_or_else(|| {
                c_size
                    .map(|cs| cs.size.width / cs.size.height)
                    .unwrap_or(1.)
            });
            let y = match y {
                Alignment::DirectMargin(m) => AxisConstraint::DirectMarginAndSize(
                    *m,
                    x.size / aspect,
                )
                .solve(solve_target.anchors.y(), space.y, None),
                Alignment::ReverseMargin(m) => AxisConstraint::ReverseMarginAndSize(
                    *m,
                    x.size / aspect,
                )
                .solve(solve_target.anchors.y(), space.y, None),
                Alignment::Offset(o) => AxisConstraintSolve {
                    offset: *o,
                    size: x.size / aspect,
                },
            };

            *target_size = Vec2::new(x.size, y.size);
            Vec2::new(x.offset, y.offset)
        }
        Constraint::MaxAspect(aspect) => {
            let x_from_y =
                (solve_target.anchors.y().1 - solve_target.anchors.y().0) * space.y * aspect;
            let y_from_x =
                (solve_target.anchors.x().1 - solve_target.anchors.x().0) * space.x / aspect;

            *target_size = if x_from_y >= space.x {
                Vec2::new(space.x, y_from_x)
            } else {
                Vec2::new(x_from_y, space.y)
            };
            Vec2::zero()
        }
    };

    if solve_target.child_constraint.is_some() {
        offset += target_transform.translation.truncate();
    };

    target_transform.translation = offset.extend(0.);


    if let Some(children) = children {
        let ts = *target_size;
        if let Some(spread_constraint) = &solve_target.children_spread {
            let child_nodes = children.iter().map(|c| {
                (
                    nodes
                        .get_component::<AnchorLayout>(*c)
                        .unwrap()
                        .child_constraint
                        .as_ref()
                        .unwrap(),
                    c,
                )
            });

            let mut free_length = match spread_constraint.direction {
                Direction::Left | Direction::Right => ts.x,
                Direction::Up | Direction::Down => ts.y,
            } - (children.iter().count() - 1) as f32
                * spread_constraint.margin;

            let mut undef = vec![];
            let mut undef_weight_sum = 0.;

            let mut locked = BTreeMap::<usize, (&Entity, f32)>::new();

            for (i, c) in child_nodes.enumerate() {
                undef_weight_sum += c.0.weight;
                undef.push((i, c));
            }

            loop {
                let mut dirty = false;
                let length_per_weight = free_length / undef_weight_sum;

                let mut k = 0;
                while k != undef.len() {
                    let (i, (n, e)) = undef[k];
                    let len = length_per_weight * n.weight;
                    if !(n.min_size..n.max_size).contains(&len) {
                        dirty = true;
                        undef_weight_sum -= n.weight;
                        let clamped = len.clamp(n.min_size, n.max_size);
                        free_length -= clamped;
                        locked.insert(i, (e, clamped));
                        undef.swap_remove(k);
                    } else {
                        k += 1;
                    }
                }

                if !dirty {
                    for (i, (n, e)) in undef.iter() {
                        let len = length_per_weight * n.weight;
                        locked.insert(*i, (e, len));
                    }
                    break;
                }
            }

            let (calc_pos, calc_size): (fn(f32, f32, Vec2) -> Vec2, fn(f32, Vec2) -> Vec2) =
                match spread_constraint.direction {
                    Direction::Up => (
                        |size, offset, ts| Vec2::new(0., offset + size / 2. - ts.y / 2.),
                        |size, ts| Vec2::new(ts.x, size),
                    ),
                    Direction::Down => (
                        |size, offset, ts| Vec2::new(0., ts.y / 2. - offset - size / 2.),
                        |size, ts| Vec2::new(ts.x, size),
                    ),
                    Direction::Left => (
                        |size, offset, ts| Vec2::new(ts.x / 2. - offset - size / 2., 0.),
                        |size, ts| Vec2::new(size, ts.y),
                    ),
                    Direction::Right => (
                        |size, offset, ts| Vec2::new(offset + size / 2. - ts.x / 2., 0.),
                        |size, ts| Vec2::new(size, ts.y),
                    ),
                };

            let mut offset = 0.;
            let mut cache = vec![];
            for &(&entity, size) in locked.values() {
                let (mut transform, _, _) = mutables.get_mut(entity).unwrap();
                transform.translation = calc_pos(size, offset, ts).extend(0.);
                offset += size + spread_constraint.margin;
                let size = calc_size(size, ts);
                cache.push(size);
                solve(entity, size, respect_flags, nodes, mutables);
            }
            let (_, _, mut target_cache) = mutables.get_mut(solve_entity).unwrap();
            target_cache.sizes = Some(cache);
        } else {
            for child in children.iter() {
                solve(*child, ts, false, nodes, mutables);
            }
        }
    }
}

impl AxisConstraint {
    fn solve(
        self,
        anchors: (f32, f32),
        true_space: f32,
        content_size: Option<f32>,
    ) -> AxisConstraintSolve {
        let space = (anchors.1 - anchors.0) * true_space;

        let (p1, s) = match self {
            AxisConstraint::DoubleMargin(p1, p2) => (p1, space - p1 - p2),
            AxisConstraint::DirectMarginAndSize(p1, s) => (p1, s),
            AxisConstraint::ReverseMarginAndSize(p2, s) => (space - p2 - s, s),
            AxisConstraint::Centered(s) => ((space - s) / 2., s),
            AxisConstraint::FromContentSize(alignment) => {
                let size = content_size.unwrap();

                match alignment {
                    Alignment::DirectMargin(v) => (v, size),
                    Alignment::ReverseMargin(v) => {
                        return AxisConstraintSolve {
                            offset: true_space * (anchors.1 - 0.5) - v - size / 2.,
                            size,
                        }
                    }
                    Alignment::Offset(offset) => {
                        let int = AxisConstraint::Centered(size).solve(anchors, true_space, None);
                        return AxisConstraintSolve {
                            offset: int.offset + offset,
                            size,
                        };
                    }
                }
            }
        };
        let offset = true_space * (anchors.0 - 0.5) + p1 + s / 2.;
        AxisConstraintSolve { offset, size: s }
    }
}

struct AxisConstraintSolve {
    offset: f32,
    size: f32,
}
