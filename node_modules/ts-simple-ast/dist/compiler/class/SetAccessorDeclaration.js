"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var base_1 = require("../base");
var callBaseFill_1 = require("../callBaseFill");
var common_1 = require("../common");
var function_1 = require("../function");
var base_2 = require("./base");
exports.SetAccessorDeclarationBase = base_1.ChildOrderableNode(base_1.TextInsertableNode(base_1.DecoratableNode(base_2.AbstractableNode(base_1.ScopedNode(base_1.StaticableNode(base_1.BodiedNode(function_1.FunctionLikeDeclaration(base_1.PropertyNamedNode(common_1.Node)))))))));
var SetAccessorDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(SetAccessorDeclaration, _super);
    function SetAccessorDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Fills the node from a structure.
     * @param structure - Structure to fill.
     */
    SetAccessorDeclaration.prototype.fill = function (structure) {
        callBaseFill_1.callBaseFill(exports.SetAccessorDeclarationBase.prototype, this, structure);
        return this;
    };
    /**
     * Gets the corresponding get accessor if one exists.
     */
    SetAccessorDeclaration.prototype.getGetAccessor = function () {
        var e_1, _a;
        var parent = this.getParentIfKindOrThrow(typescript_1.SyntaxKind.ClassDeclaration);
        var thisName = this.getName();
        try {
            for (var _b = tslib_1.__values(parent.getInstanceProperties()), _c = _b.next(); !_c.done; _c = _b.next()) {
                var prop = _c.value;
                if (prop.getKind() === typescript_1.SyntaxKind.GetAccessor && prop.getName() === thisName)
                    return prop;
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
        return undefined;
    };
    /**
     * Gets the corresponding get accessor or throws if not exists.
     */
    SetAccessorDeclaration.prototype.getGetAccessorOrThrow = function () {
        var _this = this;
        return errors.throwIfNullOrUndefined(this.getGetAccessor(), function () { return "Expected to find a corresponding get accessor for " + _this.getName() + "."; });
    };
    /**
     * Removes the set accessor.
     */
    SetAccessorDeclaration.prototype.remove = function () {
        manipulation_1.removeClassMember(this);
    };
    return SetAccessorDeclaration;
}(exports.SetAccessorDeclarationBase));
exports.SetAccessorDeclaration = SetAccessorDeclaration;
