"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var collections_1 = require("../collections");
/**
 * Updates all the source file's reference containers.
 */
var LazyReferenceCoordinator = /** @class */ (function () {
    function LazyReferenceCoordinator(factory) {
        var _this = this;
        this.dirtySourceFiles = collections_1.createHashSet();
        var onSourceFileModified = function (sourceFile) {
            if (!sourceFile.wasForgotten())
                _this.dirtySourceFiles.add(sourceFile);
        };
        factory.onSourceFileAdded(function (sourceFile) {
            _this.dirtySourceFiles.add(sourceFile);
            sourceFile.onModified(onSourceFileModified);
        });
        factory.onSourceFileRemoved(function (sourceFile) {
            sourceFile._referenceContainer.clear();
            _this.dirtySourceFiles.delete(sourceFile);
            sourceFile.onModified(onSourceFileModified, false);
        });
    }
    LazyReferenceCoordinator.prototype.refreshDirtySourceFiles = function () {
        var e_1, _a;
        try {
            for (var _b = tslib_1.__values(this.dirtySourceFiles.values()), _c = _b.next(); !_c.done; _c = _b.next()) {
                var sourceFile = _c.value;
                sourceFile._referenceContainer.refresh();
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
        this.clearDirtySourceFiles();
    };
    LazyReferenceCoordinator.prototype.refreshSourceFileIfDirty = function (sourceFile) {
        if (!this.dirtySourceFiles.has(sourceFile))
            return;
        sourceFile._referenceContainer.refresh();
        this.clearDityForSourceFile(sourceFile);
    };
    LazyReferenceCoordinator.prototype.addDirtySourceFile = function (sourceFile) {
        this.dirtySourceFiles.add(sourceFile);
    };
    LazyReferenceCoordinator.prototype.clearDirtySourceFiles = function () {
        this.dirtySourceFiles.clear();
    };
    LazyReferenceCoordinator.prototype.clearDityForSourceFile = function (sourceFile) {
        this.dirtySourceFiles.delete(sourceFile);
    };
    return LazyReferenceCoordinator;
}());
exports.LazyReferenceCoordinator = LazyReferenceCoordinator;
