"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
var Scope_1 = require("../common/Scope");
function ScopeableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getScope = function () {
            return getScopeForNode(this);
        };
        class_1.prototype.setScope = function (scope) {
            setScopeForNode(this, scope);
            return this;
        };
        class_1.prototype.hasScopeKeyword = function () {
            return this.getScope() != null;
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
exports.ScopeableNode = ScopeableNode;
/**
 * Gets the scope for a node.
 * @internal
 * @param node - Node to check for.
 */
function getScopeForNode(node) {
    var modifierFlags = node.getCombinedModifierFlags();
    if ((modifierFlags & typescript_1.ts.ModifierFlags.Private) !== 0)
        return Scope_1.Scope.Private;
    else if ((modifierFlags & typescript_1.ts.ModifierFlags.Protected) !== 0)
        return Scope_1.Scope.Protected;
    else if ((modifierFlags & typescript_1.ts.ModifierFlags.Public) !== 0)
        return Scope_1.Scope.Public;
    else
        return undefined;
}
exports.getScopeForNode = getScopeForNode;
/**
 * Sets the scope for a node.
 * @internal
 * @param node - Node to set the scope for.
 * @param scope - Scope to be set to.
 */
function setScopeForNode(node, scope) {
    node.toggleModifier("public", scope === Scope_1.Scope.Public); // always be explicit with scope
    node.toggleModifier("protected", scope === Scope_1.Scope.Protected);
    node.toggleModifier("private", scope === Scope_1.Scope.Private);
}
exports.setScopeForNode = setScopeForNode;
