pub mod spec;
pub mod policy;
pub mod guard;
pub mod topo;
pub mod engine;
pub mod stats;
pub mod flip;

pub use stats::ReductionStats;
use spec::ArithmeticSpec;
use policy::{ReductionPolicy, ReductionAction, RandomPolicy, DefaultPolicy, LazyGreedyPolicy};
use engine::ReductionEngine;
use topo::{TopoVar, VecVar, VarDomain, Domain};
use guard::{SizeGuard};
use flip::FlipManager;

use crate::{
    circuit::{Circuit, NetId, NodeId, Gate, Cone},
    bipoly::{Polynomial, VarId},
    config::{ReductionMode, Config},
    Result
};

use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use rug::Integer;
use log::debug;

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
    pub substitutions: HashMap<VarId, Substitution>
}

#[derive(Clone)]
pub struct ReductionState {
    pub poly: Polynomial,
    pub var_domain: Domain,
    pub flip_manager: Option<FlipManager>
}

// map half adder outputs to negative to make the reduction faster
pub fn init_vars(circuit: &Circuit) -> Vec<VarId> {
    let mut vars: Vec<VarId> = (0..circuit.nets().len() as i32).collect();
    circuit.nodes().iter().filter(|&node| {
        *node.gate() == Gate::HalfAdder 
    }).enumerate().for_each(|(i, node)| {
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

    for node in circuit.nodes() {
        let inputs: Vec<VarId> = node.inputs().iter().map(|lit| var[lit.net()]).collect();
        let outputs: Vec<VarId> = node.outputs().iter().map(|net| var[*net]).collect();
        
        let mut res = vec![Polynomial::new(); node.gate().n_outputs()];

        match node.gate() {
            Gate::And => { 
                res[0] += Polynomial::from_vars(vec![inputs[0], inputs[1]], one.clone());
            }
            Gate::Or => { 
                res[0] += Polynomial::from_var(inputs[0], one.clone())
                        + Polynomial::from_var(inputs[1], one.clone())
                        - Polynomial::from_vars(vec![inputs[0], inputs[1]], one.clone());
            }
            Gate::Xor => { 
                res[0] += Polynomial::from_var(inputs[0], one.clone())
                        + Polynomial::from_var(inputs[1], one.clone())
                        - Polynomial::from_vars(vec![inputs[0], inputs[1]], two.clone());
            }
            Gate::Xor3 => { 
                let sum_linear = Polynomial::from_var(inputs[0], one.clone()) + Polynomial::from_var(inputs[1], one.clone()) + Polynomial::from_var(inputs[2], one.clone());
                let sum_quad = Polynomial::from_vars(vec![inputs[0], inputs[1]], two.clone()) + Polynomial::from_vars(vec![inputs[1], inputs[2]], two.clone()) + Polynomial::from_vars(vec![inputs[0], inputs[2]], two.clone());
                let cubic = Polynomial::from_vars(vec![inputs[0], inputs[1], inputs[2]], four.clone());
                
                res[0] += sum_linear - sum_quad + cubic;
            }
            Gate::Maj => { 
                let sum_quad = Polynomial::from_vars(vec![inputs[0], inputs[1]], one.clone()) + Polynomial::from_vars(vec![inputs[1], inputs[2]], one.clone()) + Polynomial::from_vars(vec![inputs[0], inputs[2]], one.clone());
                let cubic = Polynomial::from_vars(vec![inputs[0], inputs[1], inputs[2]], two.clone());
                res[0] += sum_quad - cubic;
            }
            Gate::HalfAdder => {
                res[0] += Polynomial::from_vars(vec![inputs[0], inputs[1]], one.clone());
                res[1] += - Polynomial::from_var(outputs[0], two.clone())
                          +(Polynomial::from_var(inputs[0], one.clone())
                          + Polynomial::from_var(inputs[1], one.clone()));
            }
            Gate::FullAdder => {
                res[0] += Polynomial::from_vars(vec![inputs[0], inputs[1]], one.clone())
                        + Polynomial::from_vars(vec![inputs[0], inputs[2]], one.clone())
                        + Polynomial::from_vars(vec![inputs[1], inputs[2]], one.clone())
                        - Polynomial::from_vars(vec![inputs[0], inputs[1], inputs[2]], two.clone());
                res[1] += - Polynomial::from_var(outputs[0], two.clone())
                          +(Polynomial::from_var(inputs[0], one.clone())
                          + Polynomial::from_var(inputs[1], one.clone())
                          + Polynomial::from_var(inputs[2], one.clone()));
            }
        }

        for p in res.iter_mut() {
            for lit in node.inputs().iter() {
                if lit.negative() {
                    p.neg_var(&var[lit.net()]);
                }
            }
        }

        for (i, &net) in node.outputs().iter().enumerate() {
            poly_map.insert(var[net], res[i].clone());
        }
    }

    poly_map
}

pub fn find_ffcc(circuit: &Circuit, vars: &[VarId], sensitivity: u8) -> Vec<(Cone, bool)> {
    let terminals: HashSet<NetId> = circuit.nets().iter().enumerate().filter_map(|(net_id, net)| {
        if let Some(driver) = net.driver() {
            if circuit.nodes_at(driver).is_multioutput() {
                None
            } else if net.loads().len() > 1 || circuit.outputs().iter().find(|l| l.net() == net_id).is_some() || circuit.nodes_at(net.loads()[0]).is_multioutput(){
                Some(net_id)
            } else {
                None
            }
        } else {
            None
        }
    }).collect();
    
    let root_to_cone: HashMap<_, _> = terminals.iter().map(|&root| {
        (root, circuit.get_dfs_cone(root, |&net_id| {
            let net = circuit.nets_at(net_id);
            if net_id != root && terminals.contains(&net_id) {
                true
            } else if let Some(driver) = net.driver() {
                circuit.nodes_at(driver).is_multioutput()
            } else {
                true
            }
        }))
    }).collect();

    root_to_cone.values().map(|cone| {
        if cone.nodes.len() < 3 {
            (cone.clone(), false)
        } else {
            let mut final_nodes = Vec::new();
            let mut final_nets = Vec::new();
            let mut final_inputs = HashSet::new();

            let mut queue: Vec<&Cone> = vec![cone];

            while let Some(curr_cone) = queue.pop() { 
                for &input_net in curr_cone.inputs.iter() {
                    let should_expand = root_to_cone.get(&input_net)
                        .filter(|upstream| {
                            upstream.nodes.len() < 5
                        });
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
                .count() * 20 > final_inputs.len() * sensitivity as usize; // sensitivity: 0-10

            if is_conv {
                fn get_depth(
                    net: NetId, 
                    circuit: &Circuit, 
                    inputs: &HashSet<NetId>, 
                    depth_map: &mut HashMap<NetId, usize>
                ) -> usize {
                    if inputs.contains(&net) { return 0; }
                    if let Some(&d) = depth_map.get(&net) { return d; }
                    
                    let depth = if let Some(driver) = circuit.nets_at(net).driver() {
                        let node = circuit.nodes_at(driver);
                        let max_child_depth = node.inputs().iter()
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

                fn collect_ordered(net: NetId, circuit: &Circuit, inputs: &HashSet<NetId>, depth_map: &HashMap<NetId, usize>, acc_nets: &mut Vec<NetId>, acc_nodes: &mut Vec<NodeId>) {
                    if inputs.contains(&net) { return; }
                    
                    if let Some(driver) = circuit.nets_at(net).driver() {
                        let node = circuit.nodes_at(driver);
                        
                        let children: Vec<NetId> = node.inputs().iter().map(|l| l.net()).sorted_by_key(|c| depth_map.get(c).copied().unwrap_or(0)).collect();

                        for child in children.into_iter().rev() {
                            collect_ordered(child, circuit, inputs, depth_map, acc_nets, acc_nodes);
                        }

                        acc_nodes.push(driver);
                        acc_nets.push(net);
                    }
                }
                collect_ordered(cone.root, circuit, &final_inputs, &depth_map, &mut final_nets, &mut final_nodes);
                (Cone { root: cone.root, inputs: final_inputs.into_iter().collect(), nodes: final_nodes, nets: final_nets }, true)
            } else {
                (cone.clone(), false)
            }
        }
    }).collect()
}

fn build_cone_adj(cone: &Cone, ctx: &ReductionContext) -> HashMap<VarId, Vec<VarId>> {
    cone.nodes.iter().filter_map(|&n| {
        let node = ctx.circuit.nodes_at(n);
        if node.outputs()[0] == cone.root { return None }
        let out = ctx.vars[node.outputs()[0]];
        let ins = node.inputs().iter().filter(|l| !cone.inputs.contains(&l.net())).map(|l| ctx.vars[l.net()]).collect();
        Some((out, ins))
    }).collect()
}

pub fn process_cone(
    cone: &Cone,
    poly: Polynomial,
    is_conv: bool,
    ctx: &ReductionContext,
    size_limit: Option<usize>,
    external_flip: Option<&mut FlipManager>
) -> Result<Polynomial> {
    let run_sub = |p: Polynomial, dom: Domain, limit: usize, policy: &mut dyn ReductionPolicy| -> Result<ReductionState> {
        let state = ReductionState {
            poly: p,
            var_domain: dom,
            flip_manager: external_flip.as_ref().map(|f| (*f).clone()),
        };
        let engine = ReductionEngine::new(ctx, state, None, Some(limit));
        engine.run(policy)
    };
    let size_limit = size_limit.unwrap_or(poly.size());
    let state = if is_conv {
        let domain = Domain::Vec(VecVar::new(cone.nets[..cone.nets.len()-1].iter().map(|&n| ctx.vars[n]).collect()));
        run_sub(poly, domain, size_limit * 10, &mut DefaultPolicy {})?
    } else {
        let adj = build_cone_adj(cone, ctx);
        let topo = TopoVar::new(adj);
        run_sub(poly.clone(), Domain::Vec(VecVar::new(topo.bfs())), size_limit, &mut DefaultPolicy {})
            .or_else(|_| {
                debug!("Bfs failed, trying Dfs");
                run_sub(poly.clone(), Domain::Vec(VecVar::new(topo.dfs())), size_limit, &mut DefaultPolicy {})
            })
            .or_else(|_| {
                debug!("Dfs failed, trying Greedy");
                run_sub(poly, Domain::Topo(topo), size_limit * 10, &mut LazyGreedyPolicy::new(ctx.cfg.no_size_sort, ctx.cfg.max_ratio, ctx.cfg.abort_ratio))
            })?
    };
    if let (Some(ext), Some(sub_fm)) = (external_flip, state.flip_manager) {
        ext.update(&sub_fm);
    }
    debug!("Cone {:?} (Converging: {}): {} ", cone.root, is_conv, state.poly.size());
    Ok(state.poly)
}

pub fn verify(circuit: &Circuit, cfg: &Config, stats: &mut ReductionStats) -> Result<Polynomial> { 
    let spec = ArithmeticSpec::new(cfg.spec_str.as_deref(), cfg.signed)?;

    let vars = init_vars(circuit);
    let modulus = spec.modulus(circuit.outputs());
    let mut substitutions = HashMap::new();
    for (v, p) in init_poly_map(circuit, &vars) {
        substitutions.insert(v, Substitution::Poly(p));
    }

    let mut final_adj: HashMap<i32, Vec<i32>> = HashMap::new();

    let mut main_ctx = ReductionContext {
        circuit, cfg, vars: &vars, modulus: modulus.as_ref(),
        substitutions, 
    };

    for (cone, is_conv) in find_ffcc(circuit, &vars, cfg.revsca_sensitivity) {
        let root_var = vars[cone.root];
        final_adj.insert(root_var, cone.inputs.iter().map(|i| vars[*i]).collect());
        if cfg.delay {
            if let Some(Substitution::Poly(base_p)) = main_ctx.substitutions.get(&root_var) {
                let base_p = base_p.clone();
                main_ctx.substitutions.insert(root_var, Substitution::Cone(cone, is_conv, base_p));
            }
        } else {
            if let Some(Substitution::Poly(base_p)) = main_ctx.substitutions.remove(&root_var) {
                let optimized = process_cone(&cone, base_p, is_conv, &main_ctx, Some(cfg.size_limit), None)?;
                main_ctx.substitutions.insert(root_var, Substitution::Poly(optimized));
            }
        }
    }

    final_adj.extend(circuit.nodes().iter()
        .filter(|n| n.outputs().len() > 1) 
        .flat_map(|n| {
            let inputs: Vec<VarId> = n.inputs().iter().map(|l| vars[l.net()]).collect();
            
            n.outputs().iter().scan(inputs, |state, &key| {
                let var = vars[key];
                let vec_val = state.clone();
                state.push(var);
                Some((var, vec_val))
            })
        }));

    debug!("Start main reduction...");

    let main_state = ReductionState {
        poly: spec.build_golden(circuit.inputs(), circuit.outputs(), &vars),
        var_domain: match cfg.mode {
            ReductionMode::BFS => Domain::Vec(VecVar::new(TopoVar::new(final_adj).bfs())),
            ReductionMode::DFS => Domain::Vec(VecVar::new(TopoVar::new(final_adj).dfs())),
            ReductionMode::Random | ReductionMode::Heuristic => Domain::Topo(TopoVar::new(final_adj)),
        },
        flip_manager: if cfg.flip { Some(FlipManager::new()) } else { None },
    };

    let mut main_policy: Box<dyn ReductionPolicy> = match cfg.mode {
        ReductionMode::BFS | ReductionMode::DFS => Box::new(DefaultPolicy {}),
        ReductionMode::Random => Box::new(RandomPolicy {}),
        ReductionMode::Heuristic => Box::new(LazyGreedyPolicy::new(cfg.no_size_sort, cfg.max_ratio, cfg.abort_ratio)),
    };
    let engine = ReductionEngine::new(&main_ctx, main_state, Some(stats), Some(cfg.size_limit));
    let state = engine.run(&mut *main_policy)?;
    Ok(state.poly)
}
