//! Module implementing the logic behind board metadata
use crate::drawing::rules::AccessRules;

/// Trait for managing the Specctra's mesadata
///
/// This trait implements generic function for accessing or modifying different
/// compounds of board parts like nets or layers
pub trait AccessMesadata: AccessRules {
    /// Renames a layer based on its index.
    fn bename_layer(&mut self, layer: usize, layername: String);

    /// Retrieves the name of a layer by its index.
    fn layer_layername(&self, layer: usize) -> Option<&str>;

    /// Retrieves the index of a layer by its name.
    fn layername_layer(&self, layername: &str) -> Option<usize>;

    /// Renames a net based on its index.
    fn bename_net(&mut self, net: usize, netname: String);

    /// Retrieves the name of a net by its index.
    fn net_netname(&self, net: usize) -> Option<&str>;

    /// Retrieves the index of a net by its name.
    fn netname_net(&self, netname: &str) -> Option<usize>;
}
