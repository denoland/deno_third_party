"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var TypeElement_1 = require("./TypeElement");
exports.PropertySignatureBase = base_1.ChildOrderableNode(base_1.JSDocableNode(base_1.ReadonlyableNode(base_1.QuestionTokenableNode(base_1.InitializerExpressionableNode(base_1.TypedNode(base_1.PropertyNamedNode(base_1.ModifierableNode(TypeElement_1.TypeElement))))))));
var PropertySignature = /** @class */ (function (_super) {
    tslib_1.__extends(PropertySignature, _super);
    function PropertySignature() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    PropertySignature.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.PropertySignatureBase.prototype, this, structure);
        return this;
    };
    /**
     * Removes this property signature.
     */
    PropertySignature.prototype.remove = function () {
        manipulation_1.removeInterfaceMember(this);
    };
    return PropertySignature;
}(exports.PropertySignatureBase));
exports.PropertySignature = PropertySignature;
