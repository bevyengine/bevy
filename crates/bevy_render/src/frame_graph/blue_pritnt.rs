use super::{
    FrameGraphBuffer, FrameGraphError, PassNodeBuilder, RenderContext, ResourceRead, ResourceRef,
};

pub trait BluePrint {
    type Product;
    fn make(&self, resource_context: &RenderContext) -> Result<Self::Product, FrameGraphError>;
}

pub trait BluePrintProvider {
    type BluePrint: BluePrint;

    fn make_blue_print(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::BluePrint, FrameGraphError>;
}

impl<T: Clone + BluePrint> BluePrintProvider for T {
    type BluePrint = T;

    fn make_blue_print(
        &self,
        _pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::BluePrint, FrameGraphError> {
        Ok(self.clone())
    }
}
