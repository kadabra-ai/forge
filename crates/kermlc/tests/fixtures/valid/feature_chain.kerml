package Models {
    type Engine {
        feature cylinders : Engine;
    }
    type Vehicle {
        feature engine : Engine;
    }
    type Fleet {
        feature vehicles : Vehicle;
        feature v_engine chains vehicles.engine;
        feature v_cyl chains vehicles.engine.cylinders;
    }
}
