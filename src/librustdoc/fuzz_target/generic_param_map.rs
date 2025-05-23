use super::api_util::replace_type_with;
use super::trait_impl::{TraitImplMap,TypeTraitCache};
use crate::clean::types::{GenericArgs, Path, Type};
use crate::clean::{self, GenericBound, Generics, PolyTrait, WherePredicate};
use crate::clean::{GenericParamDefKind, Trait};
use crate::formats::cache::Cache;
use crate::fuzz_target::api_util::{_type_name, print_fact};
use crate::fuzz_target::generic_solution::{
    replace_generic_with_solution, solution_string, solution_string_with_param_name, Solution,
};
use crate::fuzz_target::impl_id::ImplId;
use crate::fuzz_target::{api_function::ApiFunction, api_util, impl_util::FullNameMap};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use std::cmp::Eq;
use std::collections::hash_map::Iter;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

pub(crate) fn set_union<T: Eq + Hash + Copy>(a: &mut FxHashSet<T>, b: &FxHashSet<T>) {
    for id in b {
        a.insert(*id);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GenericParamMap {
    pub inner: FxHashMap<String, Vec<Path>>, // generic param => bounds(a set of trait path)
    pub generic_defs: Vec<String>,
    pub type_pred: Vec<(Type, Vec<Path>)>,
}

fn is_solvable_bound(bound: &Path) -> bool {
    for segment in &bound.segments {
        if let GenericArgs::Parenthesized { .. } = segment.args {
            return false;
        }
    }
    return true;
}

fn bounds_to_vec(bounds: &[GenericBound]) -> Vec<Path> {
    let mut res = Vec::new();
    for bound in bounds {
        match bound {
            GenericBound::TraitBound(poly, _) => {
                res.push(poly.trait_.clone());
                // traitbound should not include type generic information, we must assure this
                for param in &poly.generic_params {
                    if param.is_type() {
                        unreachable!("type generic params inside traitbound!");
                    }
                }
            }
            GenericBound::Outlives(lifetime) => {
                println!("bounds to facts ignore lifetime: {:?}", lifetime);
            }
            GenericBound::Use(_use) => {
                println!("bounds to facts ignore Use: {:?}", _use);
            }
        }
    }
    res
}

impl GenericParamMap {
    pub fn iter(&self) -> Iter<'_, String, Vec<Path>> {
        self.inner.iter()
    }

    pub fn is_solvable(&self) -> bool {
        for (name, bounds) in self.inner.iter() {
            for bound in bounds {
                if !is_solvable_bound(bound) {
                    return false;
                }
            }
        }

        for (type_, bounds) in self.type_pred.iter() {
            for bound in bounds {
                if !is_solvable_bound(bound) {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn new() -> GenericParamMap {
        GenericParamMap {
            inner: FxHashMap::<String, Vec<Path>>::default(),
            generic_defs: Vec::new(),
            type_pred: Vec::new(),
        }
    }

    pub fn get_bounds(&self, name: &str) -> &Vec<Path> {
        self.inner.get(name).unwrap()
    }

    pub fn remove_type_param(&mut self, name: &str) {
        self.inner.remove(name);
    }

    pub fn set_self_type(&mut self, self_type: &Type) {
        let mut replace_self = |type_: &mut Type| -> bool {
            if type_.is_self_type() {
                *type_ = self_type.clone();
                return false;
            }
            true
        };
        for (type_, paths) in self.type_pred.iter_mut() {
            replace_type_with(type_, &mut replace_self);
            for path in paths.iter_mut() {
                let mut ty = Type::Path { path: path.clone() };
                replace_type_with(&mut ty, &mut replace_self);
                match ty {
                    Type::Path { path: pa } => *path = pa,
                    _ => unreachable!(),
                };
            }
        }
    }

    pub fn add_generics(&mut self, generics: &Generics, ignore: Option<String>) {
        for param in generics.params.iter() {
            match &param.kind {
                GenericParamDefKind::Type { bounds, default, .. } => {
                    if default.is_some() {
                        // if generic param has default value, we ignore it.
                        continue;
                    }

                    if ignore.as_ref().map_or(false,|x| x==param.name.as_str()) {
                        continue;
                    } 

                    /* if let Some(ref ignore_name) = ignore {
                        if ignore_name == param.name.as_str() {
                            continue;
                        }
                    } */
                    // println!("[add_generic]{} {:?}",param.name.as_str(), generics);
                    if !param.name.as_str().starts_with("impl ") {
                        self.add_generic_bounds(param.name.as_str(), &bounds);
                    }
                }
                GenericParamDefKind::Const { .. } => {
                    println!("ignore const: {:?}", param);
                }
                GenericParamDefKind::Lifetime { .. } => {
                    // println!("ignore lifetime: {:?}", param);
                }
            }
        }
        for param in generics.where_predicates.iter() {
            match param {
                WherePredicate::BoundPredicate { ty, bounds, bound_params } => {
                    self.type_pred.push((ty.clone(), bounds_to_vec(bounds)));
                }
                WherePredicate::RegionPredicate { lifetime, bounds } => {
                    println!("ignore RegionPredicate: {:?}", param);
                }
                WherePredicate::EqPredicate { lhs, rhs } => {
                    println!("ignore EqPredicate: {:?}", param);
                }
            }
        }
    }

    pub fn check_solution(
        &self,
        solution: &Solution,
        trait_impl_map: &TraitImplMap,
        type_trait_cache: &mut TypeTraitCache, 
        cache: &Cache,
    ) -> Option<FxHashSet<ImplId>> {
        assert!(solution.len() == self.generic_defs.len());

        let mut visited = FxHashSet::<ImplId>::default();
        for i in 0..solution.len() {
            let bounds = self.get_bounds(&self.generic_defs[i]);
            if bounds.is_empty() {
                continue;
            }

            if let Some(impl_set) = trait_impl_map.extract_type_impls_with_bounds(
                &solution[i],
                bounds,
                type_trait_cache,
                cache,
            ) {
                set_union(&mut visited, &impl_set);
            } else {
                // println!("[GenericParam] Check Pred Fail #1");
                return None;
            }
        }

        // type:
        for (type_, bounds) in self.type_pred.iter() {
            if matches!(type_, Type::QPath(_)) {
                continue; // FIXME: We currently ignore associate item
            }
            /* println!(
                "[ParamMap] check {}: {}",
                _type_name(type_, Some(cache)),
                print_fact(bounds, Some(cache))
            ); */
            let mut complete_type = type_.clone();
            replace_generic_with_solution(&mut complete_type, solution, &self.generic_defs);
            let mut complete_bounds = Vec::<Path>::new();
            for bound in bounds {
                let mut ty = Type::Path { path: bound.clone() };
                replace_generic_with_solution(&mut ty, solution, &self.generic_defs);
                match ty {
                    Type::Path { path } => complete_bounds.push(path),
                    _ => unreachable!(),
                }
            }
            if let Some(impl_set) = trait_impl_map.extract_type_impls_with_bounds(
                &complete_type,
                &complete_bounds,
                type_trait_cache,
                cache
            ) {
                set_union(&mut visited, &impl_set);
            } else {
                // println!("[GenericParam] Check Pred Fail #2");
                return None;
            }
        }

        println!(
            "[GenericParam] Check pred succ : {}",
            solution_string_with_param_name(solution, &self.generic_defs)
        );
        println!("[GenericParam] visited={:?}", visited);
        Some(visited)
    }

    pub fn add_generic_bounds(&mut self, name: &str, bounds: &[GenericBound]) {
        let v = bounds_to_vec(bounds);

        if self.inner.get(name).is_none() {
            self.inner.insert(name.to_string(), v);
        }
        self.generic_defs.push(name.to_string());
        /* if let Some(bounds) = self.inner.get_mut(name) {
            for p in v {
                bounds.push(p);
            }
        } else {
            println!("[AddBounds] {}: {:?}",name,bounds);
            self.inner.insert(name.to_string(), v);
            self.generic_defs.push(name.to_string());
        } */
    }
}

impl Deref for GenericParamMap {
    type Target = FxHashMap<String, Vec<Path>>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
