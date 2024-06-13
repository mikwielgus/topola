use crate::drawing::rules::RulesTrait;

pub trait MesadataTrait: RulesTrait {
    fn bename_layer(&mut self, layer: u64, layername: String);
    fn layer_layername(&self, layer: u64) -> Option<&str>;
    fn layername_layer(&self, layername: &str) -> Option<u64>;

    fn bename_net(&mut self, net: usize, netname: String);
    fn net_netname(&self, net: usize) -> Option<&str>;
    fn netname_net(&self, netname: &str) -> Option<usize>;
}
