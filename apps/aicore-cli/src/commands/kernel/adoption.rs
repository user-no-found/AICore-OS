#[cfg(test)]
#[path = "adoption_matrix.rs"]
mod adoption_matrix;

#[cfg(test)]
pub(crate) use adoption_matrix::{
    KernelInvocationAdoptionClass, kernel_invocation_adoption_matrix,
};
