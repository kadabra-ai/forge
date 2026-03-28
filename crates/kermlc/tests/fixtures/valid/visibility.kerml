// Tests visibility: public (default), protected, private
package Visibility {
    type Base {
        feature pub_feat : Base;
        protected feature prot_feat : Base;
        private feature priv_feat : Base;
    }

    type Sub :> Base {
        // Should inherit pub_feat and prot_feat, but NOT priv_feat
    }

    type SubSub :> Sub {
        // Should transitively inherit pub_feat and prot_feat
    }
}
