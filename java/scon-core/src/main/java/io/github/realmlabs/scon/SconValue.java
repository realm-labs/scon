package io.github.realmlabs.scon;

public sealed interface SconValue permits SconNull, SconBool, SconNumber, SconString, SconArray, SconObject {}
