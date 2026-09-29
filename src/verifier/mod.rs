pub mod engine;
pub mod flip;
pub mod guard;
pub mod meta;
pub mod policy;
pub mod spec;
pub mod topo;

use engine::ReductionEngine;
use flip::FlipManager;
use guard::SizeGuard;
use meta::ReductionMeta;
use policy::{DefaultPolicy, LazyGreedyPolicy, RandomPolicy, ReductionAction, ReductionPolicy};
use spec::ArithmeticSpec;
use topo::{Domain, TopoVar, VarDomain, VecVar};

use crate::{
    Result,
    bipoly::{Polynomial, VarId},
    circuit::{Circuit, Cone, Gate, NetId, NodeId},
    config::{Config, ReductionMode},
    json::ToJson,
};

use itertools::Itertools;
use log::debug;
use rug::Integer;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicBool, Ordering},
};

pub enum Substitution {
    Poly(Polynomial),
    Cone(Cone, bool, Polynomial),
}

impl Substitution {
    pub fn size(&self) -> usize {
        match self {
            Substitution::Poly(p) => p.size(),
            Substitution::Cone(_, _, p) => p.size(),
        }
    }
}

pub struct ReductionContext<'a> {
    pub circuit: &'a Circuit,
    pub cfg: &'a Config,
    pub vars: &'a [VarId],
    pub modulus: Option<&'a Integer>,
    pub substitutions: HashMap<VarId, Substitution>,
    pub cancelled: Option<&'a AtomicBool>,
}

impl ReductionContext<'_> {
    fn check_cancelled(&self) -> Result<()> {
        if self
            .cancelled
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err("Verification cancelled.".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct ReductionState {
    pub poly: Polynomial,
    pub var_domain: Domain,
    pub flip_manager: Option<FlipManager>,
    pub global_seq: Vec<VarId>,
    pub poly_sizes: Vec<usize>,
}

// map half adder outputs to negative to make the reduction faster
pub fn init_vars(circuit: &Circuit) -> Vec<VarId> {
    let mut vars: Vec<VarId> = (0..circuit.nets().len() as i32).collect();
    circuit
        .nodes()
        .iter()
        .filter(|&node| *node.gate() == Gate::HalfAdder)
        .enumerate()
        .for_each(|(i, node)| {
            for (j, net_id) in node.outputs().iter().enumerate() {
                vars[*net_id] = -((3 * i + 2 - j) as i32)
            }
        });
    vars
}

pub fn init_poly_map(circuit: &Circuit, var: &[VarId]) -> HashMap<VarId, Polynomial> {
    let mut poly_map = HashMap::new();

    let one = Integer::from(1);
    let two = Integer::from(2);
    let four = Integer::from(4);
    let one_poly = Polynomial::from_constant(one.clone());

    for node in circuit.nodes() {
        let outputs: Vec<VarId> = node.outputs().iter().map(|net| var[*net]).collect();
        let input_polys: Vec<Polynomial> = node
            .inputs()
            .iter()
            .map(|lit| {
                let p = Polynomial::from_var(var[lit.net()], one.clone());
                if lit.negative() { &one_poly - p } else { p }
            })
            .collect();

        let mut res = vec![Polynomial::new(); node.gate().n_outputs()];

        match node.gate() {
            Gate::And => {
                res[0] += &input_polys[0] * &input_polys[1];
            }
            Gate::Or => {
                res[0] += &input_polys[0] + &input_polys[1] - (&input_polys[0] * &input_polys[1]);
            }
            Gate::Xor => {
                res[0] += &input_polys[0]
                    + &input_polys[1]
                    - ((&input_polys[0] * &input_polys[1]) * Polynomial::from_constant(two.clone()));
            }
            Gate::Xor3 => {
                let ab = &input_polys[0] * &input_polys[1];
                let bc = &input_polys[1] * &input_polys[2];
                let ac = &input_polys[0] * &input_polys[2];
                let abc = &ab * &input_polys[2];
                let sum_linear = &input_polys[0] + &input_polys[1] + &input_polys[2];
                let sum_quad = ab * Polynomial::from_constant(two.clone())
                    + bc * Polynomial::from_constant(two.clone())
                    + ac * Polynomial::from_constant(two.clone());
                let cubic = abc * Polynomial::from_constant(four.clone());

                res[0] += sum_linear - sum_quad + cubic;
            }
            Gate::Maj => {
                let ab = &input_polys[0] * &input_polys[1];
                let bc = &input_polys[1] * &input_polys[2];
                let ac = &input_polys[0] * &input_polys[2];
                let sum_quad = ab.clone() + bc.clone() + ac.clone();
                let cubic = (&ab * &input_polys[2]) * Polynomial::from_constant(two.clone());
                res[0] += sum_quad - cubic;
            }
            Gate::HalfAdder => {
                res[0] += &input_polys[0] * &input_polys[1];
                res[1] += -Polynomial::from_var(outputs[0], two.clone())
                    + (&input_polys[0] + &input_polys[1]);
            }
            Gate::FullAdder => {
                let ab = &input_polys[0] * &input_polys[1];
                let ac = &input_polys[0] * &input_polys[2];
                let bc = &input_polys[1] * &input_polys[2];
                let abc = (&input_polys[0] * &input_polys[1]) * &input_polys[2];
                res[0] += ab + ac + bc - abc * Polynomial::from_constant(two.clone());
                res[1] += -Polynomial::from_var(outputs[0], two.clone())
                    + (&input_polys[0] + &input_polys[1] + &input_polys[2]);
            }
        }

        for (i, &net) in node.outputs().iter().enumerate() {
            poly_map.insert(var[net], res[i].clone());
        }
    }

    poly_map
}

pub fn find_ffcc(
    circuit: &Circuit,
    vars: &[VarId],
    pair_ratio_threshold: u8,
) -> Vec<(Cone, bool)> {
    let terminals: HashSet<NetId> = circuit
        .nets()
        .iter()
        .enumerate()
        .filter_map(|(net_id, net)| {
            if let Some(driver) = net.driver() {
                if *circuit.nodes_at(driver).gate() != Gate::And {
                    None
                } else if net.loads().len() != 1
                    || circuit
                        .outputs()
                        .iter()
                        .find(|l| l.net() == net_id)
                        .is_some()
                    || *circuit.nodes_at(net.loads()[0]).gate() != Gate::And
                {
                    Some(net_id)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let root_to_cone: HashMap<_, _> = terminals
        .iter()
        .map(|&root| {
            (
                root,
                circuit.get_dfs_cone(root, |&net_id| {
                    let net = circuit.nets_at(net_id);
                    if net_id != root && terminals.contains(&net_id) {
                        true
                    } else if let Some(driver) = net.driver() {
                        *circuit.nodes_at(driver).gate() != Gate::And
                    } else {
                        true
                    }
                }),
            )
        })
        .collect();

    root_to_cone
        .values()
        .map(|cone| {
            if cone.nodes.len() < 3 {
                (cone.clone(), false)
            } else {
                let mut final_nodes = Vec::new();
                let mut final_nets = Vec::new();
                let mut final_inputs = HashSet::new();

                let mut queue: Vec<&Cone> = vec![cone];

                let mut visited: HashSet<NetId> = HashSet::new();
                while let Some(curr_cone) = queue.pop() {
                    for &input_net in curr_cone.inputs.iter() {
                        if visited.contains(&input_net) { continue; }
                        visited.insert(input_net);
                        let should_expand = root_to_cone
                            .get(&input_net)
                            .filter(|upstream| upstream.nodes.len() < 5);
                        if let Some(upstream_cone) = should_expand {
                            queue.push(upstream_cone);
                        } else {
                            final_inputs.insert(input_net);
                        }
                    }
                }

                let is_conv = final_inputs
                    .iter()
                    .map(|&input| vars[input])
                    .sorted()
                    .tuple_windows()
                    .filter(|&(a, b)| b < 0 && (b - a) == 1)
                    .count()
                    * 20
                    > final_inputs.len() * pair_ratio_threshold as usize;

                if is_conv {
                    fn get_depth(
                        net: NetId,
                        circuit: &Circuit,
                        inputs: &HashSet<NetId>,
                        depth_map: &mut HashMap<NetId, usize>,
                    ) -> usize {
                        if inputs.contains(&net) {
                            return 0;
                        }
                        if let Some(&d) = depth_map.get(&net) {
                            return d;
                        }

                        let depth = if let Some(driver) = circuit.nets_at(net).driver() {
                            let node = circuit.nodes_at(driver);
                            let max_child_depth = node
                                .inputs()
                                .iter()
                                .map(|l| get_depth(l.net(), circuit, inputs, depth_map))
                                .max()
                                .unwrap_or(0);
                            max_child_depth + 1
                        } else {
                            0
                        };
                        depth_map.insert(net, depth);
                        depth
                    }

                    let mut depth_map = HashMap::new();
                    get_depth(cone.root, circuit, &final_inputs, &mut depth_map);

                    fn collect_ordered(
                        net: NetId,
                        circuit: &Circuit,
                        inputs: &HashSet<NetId>,
                        depth_map: &HashMap<NetId, usize>,
                        acc_nets: &mut Vec<NetId>,
                        acc_nodes: &mut Vec<NodeId>,
                    ) {
                        if inputs.contains(&net) {
                            return;
                        }

                        if let Some(driver) = circuit.nets_at(net).driver() {
                            let node = circuit.nodes_at(driver);

                            let children: Vec<NetId> = node
                                .inputs()
                                .iter()
                                .map(|l| l.net())
                                .sorted_by_key(|c| depth_map.get(c).copied().unwrap_or(0))
                                .collect();

                            for child in children.into_iter().rev() {
                                collect_ordered(
                                    child, circuit, inputs, depth_map, acc_nets, acc_nodes,
                                );
                            }

                            acc_nodes.push(driver);
                            acc_nets.push(net);
                        }
                    }
                    collect_ordered(
                        cone.root,
                        circuit,
                        &final_inputs,
                        &depth_map,
                        &mut final_nets,
                        &mut final_nodes,
                    );
                    (
                        Cone {
                            root: cone.root,
                            inputs: final_inputs.into_iter().collect(),
                            nodes: final_nodes,
                            nets: final_nets,
                        },
                        true,
                    )
                } else {
                    (cone.clone(), false)
                }
            }
        })
        .collect()
}

fn build_cone_adj(cone: &Cone, ctx: &ReductionContext) -> HashMap<VarId, Vec<VarId>> {
    cone.nodes
        .iter()
        .filter_map(|&n| {
            let node = ctx.circuit.nodes_at(n);
            if node.outputs()[0] == cone.root {
                return None;
            }
            let out = ctx.vars[node.outputs()[0]];
            let ins = node
                .inputs()
                .iter()
                .filter(|l| !cone.inputs.contains(&l.net()))
                .map(|l| ctx.vars[l.net()])
                .collect();
            Some((out, ins))
        })
        .collect()
}

pub fn process_cone(
    cone: &Cone,
    poly: Polynomial,
    is_conv: bool,
    ctx: &ReductionContext,
    size_limit: Option<usize>,
    external_flip: Option<&mut FlipManager>,
) -> Result<(Polynomial, Vec<VarId>, Vec<usize>)> {
    let run_sub = |p: Polynomial,
                   dom: Domain,
                   limit: Option<usize>,
                   policy: &mut dyn ReductionPolicy|
     -> Result<ReductionState> {
        let state = ReductionState {
            poly: p,
            var_domain: dom,
            flip_manager: external_flip.as_ref().map(|f| (*f).clone()),
            global_seq: Vec::new(),
            poly_sizes: Vec::new(),
        };
        let engine = ReductionEngine::new(format!("CONE_{}", cone.root), ctx, state, limit);
        engine.run(policy)
    };
    let state = if is_conv {
        let domain = Domain::Vec(VecVar::new(
            cone.nets[..cone.nets.len() - 1]
                .iter()
                .map(|&n| ctx.vars[n])
                .collect(),
        ));
        run_sub(
            poly,
            domain,
            size_limit.map(|limit| limit.saturating_mul(10)),
            &mut DefaultPolicy {},
        )?
    } else {
        let adj = build_cone_adj(cone, ctx);
        let topo = TopoVar::new(adj);
        run_sub(
            poly.clone(),
            Domain::Vec(VecVar::new(topo.bfs())),
            size_limit,
            &mut DefaultPolicy {},
        )
        .or_else(|err| {
            debug!(
                "{:<12} {:<35} | Error: {}",
                format!("[CONE_{}]", cone.root),
                "[!] BFS failed, fallback to DFS",
                err
            );
            run_sub(
                poly.clone(),
                Domain::Vec(VecVar::new(topo.dfs())),
                size_limit,
                &mut DefaultPolicy {},
            )
        })
        .or_else(|err| {
            debug!(
                "{:<12} {:<35} | Error: {}",
                format!("[CONE_{}]", cone.root),
                "[!] DFS failed, fallback to GREEDY",
                err
            );
            run_sub(
                poly,
                Domain::Topo(topo),
                size_limit.map(|limit| limit.saturating_mul(10)),
                &mut LazyGreedyPolicy::new(ctx.cfg.max_ratio),
            )
        })?
    };
    if let (Some(ext), Some(sub_fm)) = (external_flip, state.flip_manager) {
        ext.update(&sub_fm);
    }
    Ok((state.poly, state.global_seq, state.poly_sizes))
}

pub fn verify(
    circuit: &Circuit,
    cfg: &Config,
    size_limit: Option<usize>,
) -> Result<ReductionState> {
    verify_with_cancel(circuit, cfg, size_limit, None)
}

pub(crate) fn verify_with_cancel(
    circuit: &Circuit,
    cfg: &Config,
    size_limit: Option<usize>,
    cancelled: Option<&AtomicBool>,
) -> Result<ReductionState> {
    let spec = ArithmeticSpec::new(cfg.spec_str.as_deref(), cfg.signed)?;

    let vars = init_vars(circuit);
    let start_poly = spec.build_golden(circuit.inputs(), circuit.outputs(), &vars);
    let modulus = spec.modulus(circuit.outputs());
    let mut substitutions = HashMap::new();
    for (v, p) in init_poly_map(circuit, &vars) {
        substitutions.insert(v, Substitution::Poly(p));
    }

    let mut final_adj: HashMap<i32, Vec<i32>> = HashMap::new();

    let mut main_ctx = ReductionContext {
        circuit,
        cfg,
        vars: &vars,
        modulus: modulus.as_ref(),
        substitutions,
        cancelled,
    };

    for (cone, is_conv) in find_ffcc(
        circuit,
        &vars,
        cfg.cone_expansion.pair_ratio_threshold(),
    ) {
        let root_var = vars[cone.root];
        final_adj.insert(root_var, cone.inputs.iter().map(|i| vars[*i]).collect());
        if cfg.delay {
            if let Some(Substitution::Poly(base_p)) = main_ctx.substitutions.get(&root_var) {
                let base_p = base_p.clone();
                main_ctx
                    .substitutions
                    .insert(root_var, Substitution::Cone(cone, is_conv, base_p));
            }
        } else {
            if let Some(Substitution::Poly(base_p)) = main_ctx.substitutions.remove(&root_var) {
                let (optimized, _, _) = process_cone(
                    &cone,
                    base_p,
                    is_conv,
                    &main_ctx,
                    size_limit,
                    None,
                )?;
                main_ctx
                    .substitutions
                    .insert(root_var, Substitution::Poly(optimized));
            }
        }
    }

    final_adj.extend(
        circuit
            .nodes()
            .iter()
            .filter(|n| *n.gate() != Gate::And)
            .flat_map(|n| {
                let inputs: Vec<VarId> = n.inputs().iter().map(|l| vars[l.net()]).collect();

                n.outputs().iter().scan(inputs, |state, &key| {
                    let var = vars[key];
                    let vec_val = state.clone();
                    state.push(var);
                    Some((var, vec_val))
                })
            }),
    );

    let main_state = ReductionState {
        poly_sizes: vec![start_poly.size()],
        poly: start_poly,
        var_domain: match cfg.mode {
            ReductionMode::BFS => Domain::Vec(VecVar::new(TopoVar::new(final_adj).bfs())),
            ReductionMode::DFS => Domain::Vec(VecVar::new(TopoVar::new(final_adj).dfs())),
            ReductionMode::Random | ReductionMode::Heuristic => {
                Domain::Topo(TopoVar::new(final_adj))
            }
        },
        flip_manager: if cfg.flip {
            Some(FlipManager::new())
        } else {
            None
        },
        global_seq: Vec::new(),
    };

    let mut main_policy: Box<dyn ReductionPolicy> = match cfg.mode {
        ReductionMode::BFS | ReductionMode::DFS => Box::new(DefaultPolicy {}),
        ReductionMode::Random => Box::new(RandomPolicy {}),
        ReductionMode::Heuristic => Box::new(LazyGreedyPolicy::new(cfg.max_ratio)),
    };
    let engine = ReductionEngine::new(
        "MAIN",
        &main_ctx,
        main_state,
        size_limit,
    );
    let state = engine.run(&mut *main_policy)?;

    if let Some(out_path) = &cfg.meta_file {
        ReductionMeta::new(&vars, &state.global_seq, &state.poly_sizes).write_json(out_path)?;
    }

    Ok(state)
}
