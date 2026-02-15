package Lib {
    type Base {}
}

package App {
    import Lib::*;
    type Widget :> Base {}
}
