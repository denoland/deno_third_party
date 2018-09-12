"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var TypeElement_1 = require("./TypeElement");
exports.CallSignatureDeclarationBase = base_1.TypeParameteredNode(base_1.ChildOrderableNode(base_1.JSDocableNode(base_1.SignaturedDeclaration(TypeElement_1.TypeElement))));
var CallSignatureDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(CallSignatureDeclaration, _super);
    function CallSignatureDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    CallSignatureDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.CallSignatureDeclarationBase.prototype, this, structure);
        return this;
    };
    /**
     * Removes this call signature.
     */
    CallSignatureDeclaration.prototype.remove = function () {
        manipulation_1.removeInterfaceMember(this);
    };
    return CallSignatureDeclaration;
}(exports.CallSignatureDeclarationBase));
exports.CallSignatureDeclaration = CallSignatureDeclaration;
