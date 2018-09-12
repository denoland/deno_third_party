"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var callBaseFill_1 = require("../callBaseFill");
var Scope_1 = require("../common/Scope");
var scopeableNode = require("./ScopeableNode");
function ScopedNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getScope = function () {
            return scopeableNode.getScopeForNode(this) || Scope_1.Scope.Public;
        };
        class_1.prototype.setScope = function (scope) {
            scopeableNode.setScopeForNode(this, scope);
            return this;
        };
        class_1.prototype.hasScopeKeyword = function () {
            return scopeableNode.getScopeForNode(this) != null;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.scope != null)
                this.setScope(structure.scope);
            return this;
        };
        return class_1;
    }(Base));
}
exports.ScopedNode = ScopedNode;
