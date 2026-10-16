use crate::{GuardAssertion, GuardFact, GuardFacts};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// 判断规则是否有足够覆盖；缺失覆盖返回原因，不猜测不存在的边。
pub(crate) fn coverage_problem(
    assertion: &GuardAssertion,
    facts: &GuardFacts,
    required: bool,
) -> Option<String> {
    // v1alpha1 保持原有精确关系语义；严格覆盖门禁仅适用于显式升级的契约。
    if !required {
        return None;
    }
    let Some(coverage) = &facts.coverage else {
        return required.then(|| "v1alpha2 rules require explicit analyzer coverage".into());
    };
    if !coverage
        .predicates
        .iter()
        .any(|p| p == assertion.predicate())
    {
        return Some(format!("unsupported predicate: {}", assertion.predicate()));
    }
    let known: HashSet<&str> = coverage.entities.iter().map(String::as_str).collect();
    for entity in assertion.entities() {
        if !known.contains(entity) {
            return Some(format!("entity is not covered: {entity}"));
        }
    }
    None
}

/// 返回是否违反及真实见证边；facts 已经排序，路径选择可复现。
pub(crate) fn violation(assertion: &GuardAssertion, facts: &[GuardFact]) -> (bool, Vec<GuardFact>) {
    let edges: Vec<&GuardFact> = facts
        .iter()
        .filter(|f| f.predicate == assertion.predicate())
        .collect();
    match assertion {
        GuardAssertion::ForbidRelation {
            subject, object, ..
        }
        | GuardAssertion::RequireRelation {
            subject, object, ..
        } => {
            let found: Vec<GuardFact> = edges
                .iter()
                .filter(|f| &f.subject == subject && &f.object == object)
                .map(|f| (*f).clone())
                .collect();
            let violated = if matches!(assertion, GuardAssertion::RequireRelation { .. }) {
                found.is_empty()
            } else {
                !found.is_empty()
            };
            (violated, if violated { found } else { vec![] })
        }
        GuardAssertion::AllowedDependencies {
            subject, objects, ..
        } => {
            let allowed: HashSet<&str> = objects.iter().map(String::as_str).collect();
            let found: Vec<GuardFact> = edges
                .iter()
                .filter(|f| &f.subject == subject && !allowed.contains(f.object.as_str()))
                .map(|f| (*f).clone())
                .collect();
            (!found.is_empty(), found)
        }
        GuardAssertion::ForbidPath {
            subject, object, ..
        } => {
            let graph = adjacency(&edges);
            let mut queue = VecDeque::from([subject.as_str()]);
            let mut seen = HashSet::from([subject.as_str()]);
            let mut parents: HashMap<&str, &GuardFact> = HashMap::new();
            while let Some(node) = queue.pop_front() {
                for edge in graph.get(node).into_iter().flatten() {
                    if &edge.object == object {
                        let mut path = vec![(*edge).clone()];
                        let mut at = node;
                        while at != subject {
                            let prior = parents[at];
                            path.push(prior.clone());
                            at = &prior.subject;
                        }
                        path.reverse();
                        return (true, path);
                    }
                    if seen.insert(&edge.object) {
                        parents.insert(&edge.object, edge);
                        queue.push_back(&edge.object);
                    }
                }
            }
            (false, vec![])
        }
        GuardAssertion::Acyclic { .. } => {
            let graph = adjacency(&edges);
            let mut finished = HashSet::new();
            for start in graph.keys() {
                if finished.contains(start) {
                    continue;
                }
                let mut stack = vec![(*start, 0usize)];
                let mut active = HashMap::from([(*start, 0usize)]);
                let mut path: Vec<GuardFact> = vec![];
                while let Some((node, index)) = stack.last_mut() {
                    let outgoing = graph.get(node).map(Vec::as_slice).unwrap_or(&[]);
                    if *index == outgoing.len() {
                        finished.insert(*node);
                        active.remove(node);
                        stack.pop();
                        if !path.is_empty() {
                            path.pop();
                        }
                        continue;
                    }
                    let edge = outgoing[*index];
                    *index += 1;
                    if let Some(&begin) = active.get(edge.object.as_str()) {
                        let mut cycle = path[begin..].to_vec();
                        cycle.push(edge.clone());
                        return (true, cycle);
                    }
                    if !finished.contains(edge.object.as_str()) {
                        active.insert(edge.object.as_str(), stack.len());
                        path.push(edge.clone());
                        stack.push((&edge.object, 0));
                    }
                }
            }
            (false, vec![])
        }
    }
}

fn adjacency<'a>(edges: &[&'a GuardFact]) -> BTreeMap<&'a str, Vec<&'a GuardFact>> {
    let mut graph: BTreeMap<&str, Vec<&GuardFact>> = BTreeMap::new();
    for edge in edges {
        graph.entry(&edge.subject).or_default().push(edge);
    }
    graph
}
