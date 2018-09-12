"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../errors");
var typescript_1 = require("../typescript");
var utils_1 = require("../utils");
/**
 * Extension of KeyValueCache that allows for "forget points."
 */
var ForgetfulNodeCache = /** @class */ (function (_super) {
    tslib_1.__extends(ForgetfulNodeCache, _super);
    function ForgetfulNodeCache() {
        var _this = _super !== null && _super.apply(this, arguments) || this;
        _this.forgetStack = [];
        return _this;
    }
    ForgetfulNodeCache.prototype.getOrCreate = function (key, createFunc) {
        var _this = this;
        return _super.prototype.getOrCreate.call(this, key, function () {
            var node = createFunc();
            if (_this.forgetStack.length > 0)
                _this.forgetStack[_this.forgetStack.length - 1].add(node);
            return node;
        });
    };
    ForgetfulNodeCache.prototype.setForgetPoint = function () {
        this.forgetStack.push(utils_1.createHashSet());
    };
    ForgetfulNodeCache.prototype.forgetLastPoint = function () {
        var nodes = this.forgetStack.pop();
        if (nodes != null)
            this.forgetNodes(nodes.values());
    };
    ForgetfulNodeCache.prototype.rememberNode = function (node) {
        var e_1, _a;
        if (node.wasForgotten())
            throw new errors.InvalidOperationError("Cannot remember a node that was removed or forgotten.");
        var wasInForgetStack = false;
        try {
            for (var _b = tslib_1.__values(this.forgetStack), _c = _b.next(); !_c.done; _c = _b.next()) {
                var stackItem = _c.value;
                if (stackItem.delete(node)) {
                    wasInForgetStack = true;
                    break;
                }
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
        if (wasInForgetStack)
            this.rememberParentOfNode(node);
        return wasInForgetStack;
    };
    ForgetfulNodeCache.prototype.rememberParentOfNode = function (node) {
        var parent = node.getParentSyntaxList() || node.getParent();
        if (parent != null)
            this.rememberNode(parent);
    };
    ForgetfulNodeCache.prototype.forgetNodes = function (nodes) {
        var e_2, _a;
        try {
            for (var nodes_1 = tslib_1.__values(nodes), nodes_1_1 = nodes_1.next(); !nodes_1_1.done; nodes_1_1 = nodes_1.next()) {
                var node = nodes_1_1.value;
                if (node.wasForgotten() || node.getKind() === typescript_1.SyntaxKind.SourceFile)
                    continue;
                node.forgetOnlyThis();
            }
        }
        catch (e_2_1) { e_2 = { error: e_2_1 }; }
        finally {
            try {
                if (nodes_1_1 && !nodes_1_1.done && (_a = nodes_1.return)) _a.call(nodes_1);
            }
            finally { if (e_2) throw e_2.error; }
        }
    };
    return ForgetfulNodeCache;
}(utils_1.KeyValueCache));
exports.ForgetfulNodeCache = ForgetfulNodeCache;
