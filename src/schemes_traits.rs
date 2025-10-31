use bevy::prelude::*;

pub trait TnuaScheme: 'static + Send + Sync {
    type Basis: Tnua2Basis;
    type Config: TnuaSchemeConfig<Scheme = Self> + Asset;
}

pub trait TnuaSchemeConfig {
    type Scheme: TnuaScheme<Config = Self>;

    fn basis_config(&self) -> &<<Self::Scheme as TnuaScheme>::Basis as Tnua2Basis>::Config;
}

pub trait Tnua2Basis: Default + 'static + Send + Sync {
    type Config: Clone;
    type Memory;
}
