use crate::drawing::rules::AccessRules;

pub trait AccessMesadata: AccessRules {
    fn bename_layer(&mut self, layer: usize, layername: String);
    fn layer_layername(&self, layer: usize) -> Option<&str>;
    fn layername_layer(&self, layername: &str) -> Option<usize>;

    fn bename_net(&mut self, net: usize, netname: String);
    fn net_netname(&self, net: usize) -> Option<&str>;
    fn netname_net(&self, netname: &str) -> Option<usize>;
}
