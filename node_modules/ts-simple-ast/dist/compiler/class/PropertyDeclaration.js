"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var common_1 = require("../common");
var base_2 = require("./base");
exports.PropertyDeclarationBase = base_1.ChildOrderableNode(base_1.DecoratableNode(base_2.AbstractableNode(base_1.ScopedNode(base_1.StaticableNode(base_1.JSDocableNode(base_1.ReadonlyableNode(base_1.ExclamationTokenableNode(base_1.QuestionTokenableNode(base_1.InitializerExpressionableNode(base_1.TypedNode(base_1.PropertyNamedNode(base_1.ModifierableNode(common_1.Node)))))))))))));
var PropertyDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(PropertyDeclaration, _super);
    function PropertyDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    PropertyDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.PropertyDeclarationBase.prototype, this, structure);
        return this;
    };
    /**
     * Removes the property.
     */
    PropertyDeclaration.prototype.remove = function () {
        var parent = this.getParentOrThrow();
        switch (parent.getKind()) {
            case typescript_1.SyntaxKind.ClassDeclaration:
                manipulation_1.removeClassMember(this);
                break;
            default:
                throw new errors.NotImplementedError("Not implemented parent syntax kind: " + parent.getKindName());
        }
    };
    return PropertyDeclaration;
}(exports.PropertyDeclarationBase));
exports.PropertyDeclaration = PropertyDeclaration;
