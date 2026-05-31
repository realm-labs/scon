package io.github.realmlabs.scon;

import java.util.ArrayList;
import java.util.Collection;

public final class SconArray extends ArrayList<SconValue> implements SconValue {
    public SconArray() {}

    public SconArray(Collection<? extends SconValue> values) {
        super(values);
    }
}
