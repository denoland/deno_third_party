"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var TypeElement_1 = require("./TypeElement");
exports.MethodSignatureBase = base_1.ChildOrderableNode(base_1.JSDocableNode(base_1.QuestionTokenableNode(base_1.TypeParameteredNode(base_1.SignaturedDeclaration(base_1.PropertyNamedNode(TypeElement_1.TypeElement))))));
var MethodSignature = /** @class */ (function (_super) {
    tslib_1.__extends(MethodSignature, _super);
    function MethodSignature() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    MethodSignature.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.MethodSignatureBase.prototype, this, structure);
        return this;
    };
    /**
     * Removes this method signature.
     */
    MethodSignature.prototype.remove = function () {
        manipulation_1.removeInterfaceMember(this);
    };
    return MethodSignature;
}(exports.MethodSignatureBase));
exports.MethodSignature = MethodSignature;
