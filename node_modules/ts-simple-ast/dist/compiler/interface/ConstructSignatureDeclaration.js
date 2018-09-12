"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var TypeElement_1 = require("./TypeElement");
exports.ConstructSignatureDeclarationBase = base_1.TypeParameteredNode(base_1.ChildOrderableNode(base_1.JSDocableNode(base_1.SignaturedDeclaration(TypeElement_1.TypeElement))));
var ConstructSignatureDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(ConstructSignatureDeclaration, _super);
    function ConstructSignatureDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    ConstructSignatureDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.ConstructSignatureDeclarationBase.prototype, this, structure);
        return this;
    };
    /**
     * Removes this construct signature.
     */
    ConstructSignatureDeclaration.prototype.remove = function () {
        manipulation_1.removeInterfaceMember(this);
    };
    return ConstructSignatureDeclaration;
}(exports.ConstructSignatureDeclarationBase));
exports.ConstructSignatureDeclaration = ConstructSignatureDeclaration;
