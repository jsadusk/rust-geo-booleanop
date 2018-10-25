use super::divide_segment::divide_segment;
use super::segment_intersection::{intersection, LineIntersection};
use super::sweep_event::{EdgeType, SweepEvent};
use num_traits::Float;
use std::collections::BinaryHeap;
use std::rc::Rc;

pub fn possible_intersection<F>(
    se1: Rc<SweepEvent<F>>,
    se2: Rc<SweepEvent<F>>,
    queue: &mut BinaryHeap<Rc<SweepEvent<F>>>,
) -> u8
where
    F: Float,
{
    let (other1, other2) = match (se1.get_other_event(), se2.get_other_event()) {
        (Some(other1), Some(other2)) => (other1, other2),
        _ => return 0,
    };

    match intersection(se1.point, other1.point, se2.point, other2.point) {
        LineIntersection::None => 0, // No intersection
        LineIntersection::Point(_) if se1.point == se2.point && other1.point == other2.point => 0, // the line segments intersect at an endpoint of both line segments
        LineIntersection::Point(inter) => {
            if se1.point != inter && other1.point != inter {
                divide_segment(&se1, inter, queue)
            }
            if se2.point != inter && other2.point != inter {
                divide_segment(&se2, inter, queue)
            }
            1
        }
        LineIntersection::Overlap(_, _) if se1.is_subject == se2.is_subject => 0, // The line segments associated to se1 and se2 overlap
        LineIntersection::Overlap(_, _) => {
            let mut events = Vec::new();
            let mut left_coincide = false;
            let mut right_coincide = false;

            if se1.point == se2.point {
                left_coincide = true
            } else if se1 < se2 {
                events.push((se2.clone(), other2.clone()));
                events.push((se1.clone(), other1.clone()));
            } else {
                events.push((se1.clone(), other1.clone()));
                events.push((se2.clone(), other2.clone()));
            }

            if other1.point == other2.point {
                right_coincide = true
            } else if other1 < other2 {
                events.push((other2.clone(), se2.clone()));
                events.push((other1.clone(), se1.clone()));
            } else {
                events.push((other1.clone(), se1.clone()));
                events.push((other2.clone(), se2.clone()));
            }

            if left_coincide {
                // both line segments are equal or share the left endpoint
                se2.set_edge_type(EdgeType::NonContributing);
                if se1.is_in_out() == se2.is_in_out() {
                    se1.set_edge_type(EdgeType::SameTransition)
                } else {
                    se1.set_edge_type(EdgeType::DifferentTransition)
                }

                if left_coincide && !right_coincide {
                    divide_segment(&events[1].1, events[0].0.point, queue)
                }
                return 2;
            }

            if right_coincide {
                // the line segments share the right endpoint
                divide_segment(&events[0].0, events[1].0.point, queue);
                return 3;
            }

            if !Rc::ptr_eq(&events[0].0, &events[3].1) {
                // no line segment includes totally the other one
                divide_segment(&events[0].0, events[1].0.point, queue);
                divide_segment(&events[1].0, events[2].0.point, queue);
                return 3;
            }

            // one line segment includes the other one
            divide_segment(&events[0].0, events[1].0.point, queue);
            divide_segment(&events[3].1, events[2].0.point, queue);

            3
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::compare_segments::compare_segments;
    use super::super::fill_queue::fill_queue;
    use super::super::helper::test::fixture_shapes;
    use super::super::subdivide_segments::subdivide;
    use super::super::sweep_event::SweepEvent;
    use super::super::Operation;
    use super::*;
    use geo::{Coordinate, Rect};
    use splay::SplaySet;
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    use std::rc::{Rc, Weak};

    fn make_simple(
        a: Coordinate<f64>,
        b: Coordinate<f64>,
        is_subject: bool,
    ) -> (Rc<SweepEvent<f64>>, Rc<SweepEvent<f64>>) {
        let other = SweepEvent::new(0, b, false, Weak::new(), is_subject, true);
        let event = SweepEvent::new(0, a, true, Rc::downgrade(&other), is_subject, true);

        (event, other)
    }

    #[test]
    fn test_possible_intersection() {
        let (s, c) = fixture_shapes("two_shapes.geojson");
        let mut q: BinaryHeap<Rc<SweepEvent<f64>>> = BinaryHeap::new();

        let (se1, _other1) = make_simple(s.exterior.0[3], s.exterior.0[2], true);
        let (se2, _other2) = make_simple(c.exterior.0[0], c.exterior.0[1], false);

        assert_eq!(possible_intersection(se1.clone(), se2.clone(), &mut q), 1);
        assert_eq!(q.len(), 4);

        let mut e = q.pop().unwrap();
        assert_eq!(
            e.point,
            Coordinate {
                x: 100.79403384562251,
                y: 233.41363754101192
            }
        );
        assert_eq!(e.get_other_event().unwrap().point, Coordinate { x: 56.0, y: 181.0 });

        e = q.pop().unwrap();
        assert_eq!(
            e.point,
            Coordinate {
                x: 100.79403384562251,
                y: 233.41363754101192
            }
        );
        assert_eq!(e.get_other_event().unwrap().point, Coordinate { x: 16.0, y: 282.0 });

        e = q.pop().unwrap();
        assert_eq!(
            e.point,
            Coordinate {
                x: 100.79403384562251,
                y: 233.41363754101192
            }
        );
        assert_eq!(e.get_other_event().unwrap().point, Coordinate { x: 153.0, y: 203.5 });

        e = q.pop().unwrap();
        assert_eq!(
            e.point,
            Coordinate {
                x: 100.79403384562251,
                y: 233.41363754101192
            }
        );
        assert_eq!(e.get_other_event().unwrap().point, Coordinate { x: 153.0, y: 294.5 });
    }

    #[test]
    fn test_on_two_polygons() {
        let (s, c) = fixture_shapes("two_shapes.geojson");
        let mut sbbox = Rect {
            min: Coordinate {
                x: f64::infinity(),
                y: f64::infinity(),
            },
            max: Coordinate {
                x: f64::neg_infinity(),
                y: f64::neg_infinity(),
            },
        };
        let mut cbbox = sbbox;
        let mut q = fill_queue(&[s], &[c], &mut sbbox, &mut cbbox, Operation::Intersection);

        let p0 = Coordinate { x: 16.0, y: 282.0 };
        let p1 = Coordinate { x: 298.0, y: 359.0 };
        let p2 = Coordinate { x: 156.0, y: 203.5 };

        let te = SweepEvent::new(0, p0, true, Weak::new(), true, true);
        let te2 = SweepEvent::new(0, p1, false, Rc::downgrade(&te), false, true);
        te.set_other_event(&te2);

        let te3 = SweepEvent::new(0, p0, true, Weak::new(), true, true);
        let te4 = SweepEvent::new(0, p2, true, Rc::downgrade(&te3), false, true);
        te3.set_other_event(&te4);

        let mut tr = SplaySet::new(compare_segments);

        tr.insert(te.clone());
        tr.insert(te3.clone());

        assert!(Rc::ptr_eq(&te, tr.find(&te).unwrap()));
        assert!(Rc::ptr_eq(&te3, tr.find(&te3).unwrap()));

        assert_eq!(compare_segments(&te, &te3), Ordering::Greater);
        assert_eq!(compare_segments(&te3, &te), Ordering::Less);

        let segments = subdivide(&mut q, &sbbox, &cbbox, Operation::Intersection);

        let left_segments = segments.iter().filter(|s| s.is_left()).cloned().collect::<Vec<_>>();

        assert_eq!(left_segments.len(), 11);

        let e = Coordinate::<f64> { x: 16.0, y: 282.0 };
        let i = Coordinate::<f64> {
            x: 100.79403384562252,
            y: 233.41363754101192,
        };
        let g = Coordinate::<f64> { x: 298.0, y: 359.0 };
        let c = Coordinate::<f64> { x: 153.0, y: 294.5 };
        let j = Coordinate::<f64> {
            x: 203.36313843035356,
            y: 257.5101243166895,
        };
        let f = Coordinate::<f64> { x: 153.0, y: 203.5 };
        let d = Coordinate::<f64> { x: 56.0, y: 181.0 };
        let a = Coordinate::<f64> { x: 108.5, y: 120.0 };
        let b = Coordinate::<f64> { x: 241.5, y: 229.5 };

        let intervals = &[
            ("EI", e, i, false, true, false),
            ("IF", i, f, false, false, true),
            ("FJ", f, j, false, false, true),
            ("JG", j, g, false, true, false),
            ("EG", e, g, true, true, false),
            ("DA", d, a, false, true, false),
            ("AB", a, b, false, true, false),
            ("JB", j, b, true, true, false),
            ("CJ", c, j, true, false, true),
            ("IC", i, c, true, false, true),
            ("DC", d, i, true, true, false),
        ];

        for (interval, a, b, in_out, other_in_out, in_result) in intervals {
            let mut found = false;

            for segment in &left_segments {
                if segment.point == *a
                    && segment.get_other_event().unwrap().point == *b
                    && segment.is_in_out() == *in_out
                    && segment.is_other_in_out() == *other_in_out
                    && segment.is_in_result() == *in_result
                {
                    found = true;
                    break;
                }
            }
            if !found {
                panic!(format!("interval {} not found", interval))
            }
        }
    }
}
