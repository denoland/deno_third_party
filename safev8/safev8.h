#ifndef SAFE_V8_H_
#define SAFE_V8_H_

#include <functional>
#include "v8.h"

namespace safeV8 {

using namespace v8;

/////// File Contents //////////////////////////////////////////////////////////////////////////////////////////////
// Some template helpers to extract types of parameter and return types of lambdas
// Internal Conversion Functions
// Common Base class definition
// Monadic function structure for marshal API (i.e. if a JS object points to an array, attempt to convert the type to an array)
// Monadic function structure for
//    getProp API (i.e. get a particular field from a json object)
//    hasProp API (i.e. check for a particular field from a json object)
//    delProp API (i.e. delete a particular field from a json object)
//    hasOwnProp API (i.e. check for a particular field from a json object, without going up the inheritence tree)
//    getOwnPropDescriptor API
// Monadic function structure for setProp API (i.e. set a particular field from a json object)
// Monadic function structure for
//    toString API (i.e. stringify object)
//    getPropNames API (i.e. get the list properties on this json object)
//    getOwnPropNames API (i.e. get the list properties on this json object, without going up the inheritence tree)
// Monadic function structure for implicitCoerce Unsafe casting APIs - such as converting to bool, double, uint32_t, int32_t
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////



/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Some template helpers to extract types of parameter and return types of lambdas
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

  /* Returns the first argument type of a given lambda */

  template<typename Ret, typename Arg, typename... Rest>
  Arg first_argument_helper(Ret(*) (Arg, Rest...));

  template<typename Ret, typename F, typename Arg, typename... Rest>
  Arg first_argument_helper(Ret(F::*) (Arg, Rest...));

  template<typename Ret, typename F, typename Arg, typename... Rest>
  Arg first_argument_helper(Ret(F::*) (Arg, Rest...) const);

  template <typename F>
  decltype(first_argument_helper(&F::operator())) first_argument_helper(F);

  template <typename T>
  using first_argument = decltype(first_argument_helper(std::declval<T>()));

  /* Returns the second argument type of a given lambda */

  template<typename Ret, typename Arg, typename Arg2, typename... Rest>
  Arg2 second_argument_helper(Ret(*) (Arg, Arg2, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename... Rest>
  Arg2 second_argument_helper(Ret(F::*) (Arg, Arg2, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename... Rest>
  Arg2 second_argument_helper(Ret(F::*) (Arg, Arg2, Rest...) const);

  template <typename F>
  decltype(second_argument_helper(&F::operator())) second_argument_helper(F);

  template <typename T>
  using second_argument = decltype(second_argument_helper(std::declval<T>()));

  /* Returns the third argument type of a given lambda */

  template<typename Ret, typename Arg, typename Arg2, typename Arg3, typename... Rest>
  Arg3 third_argument_helper(Ret(*) (Arg, Arg2, Arg3, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename... Rest>
  Arg3 third_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename... Rest>
  Arg3 third_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Rest...) const);

  template <typename F>
  decltype(third_argument_helper(&F::operator())) third_argument_helper(F);

  template <typename T>
  using third_argument = decltype(third_argument_helper(std::declval<T>()));

  /* Returns the fourth argument type of a given lambda */

  template<typename Ret, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename... Rest>
  Arg4 fourth_argument_helper(Ret(*) (Arg, Arg2, Arg3, Arg4, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename... Rest>
  Arg4 fourth_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Arg4, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename... Rest>
  Arg4 fourth_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Arg4, Rest...) const);

  template <typename F>
  decltype(fourth_argument_helper(&F::operator())) fourth_argument_helper(F);

  template <typename T>
  using fourth_argument = decltype(fourth_argument_helper(std::declval<T>()));

  /* Returns the fifth argument type of a given lambda */

  template<typename Ret, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename Arg5, typename... Rest>
  Arg5 fifth_argument_helper(Ret(*) (Arg, Arg2, Arg3, Arg4, Arg5, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename Arg5, typename... Rest>
  Arg5 fifth_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Arg4, Arg5, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename Arg5, typename... Rest>
  Arg5 fifth_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Arg4, Arg5, Rest...) const);

  template <typename F>
  decltype(fifth_argument_helper(&F::operator())) fifth_argument_helper(F);

  template <typename T>
  using fifth_argument = decltype(fifth_argument_helper(std::declval<T>()));

  /* Returns the sixth argument type of a given lambda */

  template<typename Ret, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename Arg5, typename Arg6, typename... Rest>
  Arg6 sixth_argument_helper(Ret(*) (Arg, Arg2, Arg3, Arg4, Arg5, Arg6, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename Arg5, typename Arg6, typename... Rest>
  Arg6 sixth_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Arg4, Arg5, Arg6, Rest...));

  template<typename Ret, typename F, typename Arg, typename Arg2, typename Arg3, typename Arg4, typename Arg5, typename Arg6, typename... Rest>
  Arg6 sixth_argument_helper(Ret(F::*) (Arg, Arg2, Arg3, Arg4, Arg5, Arg6, Rest...) const);

  template <typename F>
  decltype(sixth_argument_helper(&F::operator())) sixth_argument_helper(F);

  template <typename T>
  using sixth_argument = decltype(sixth_argument_helper(std::declval<T>()));


  /* Returns the return argument type of a given lambda */

  template<typename Ret, typename... Rest>
  Ret return_argument_helper(Ret(*) (Rest...));

  template<typename Ret, typename F, typename... Rest>
  Ret return_argument_helper(Ret(F::*) (Rest...));

  template<typename Ret, typename F, typename... Rest>
  Ret return_argument_helper(Ret(F::*) (Rest...) const);

  template <typename F>
  decltype(return_argument_helper(&F::operator())) return_argument_helper(F);

  template <typename T>
  using return_argument = decltype(return_argument_helper(std::declval<T>()));

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal Conversion Functions
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#define DEFINE_TY_VAL(Type) \
  inline bool SafeV8ConvertVal(Isolate* isolate, Local<Value> v, Local<v8::Type>& outVal, Local<Value>& err, bool& hasError) { \
    if (v->Is##Type()) { \
      outVal = v.As<v8::Type>(); \
      hasError = false; \
      return true; \
    } else { \
      MaybeLocal<String> mErrMsg =  \
        v8::String::NewFromUtf8(isolate, "Invalid type", v8::String::NewStringType::kNormalString); \
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked()); \
      hasError = true; \
      return false; \
    } \
  }

#define TYPE_LIST(V)   \
  V(Array)             \
  V(ArrayBuffer)       \
  V(ArrayBufferView)   \
  V(Boolean)           \
  V(DataView)          \
  V(Date)              \
  V(External)          \
  V(Float32Array)      \
  V(Float64Array)      \
  V(Function)          \
  V(Int16Array)        \
  V(Int32)             \
  V(Int32Array)        \
  V(Int8Array)         \
  V(Map)               \
  V(Name)              \
  V(Number)            \
  V(Object)            \
  V(Proxy)             \
  V(RegExp)            \
  V(Set)               \
  V(SharedArrayBuffer) \
  V(String)            \
  V(StringObject)      \
  V(Symbol)            \
  V(TypedArray)        \
  V(Uint16Array)       \
  V(Uint32)            \
  V(Uint32Array)       \
  V(Uint8Array)        \
  V(Uint8ClampedArray)

  TYPE_LIST(DEFINE_TY_VAL)

#undef TYPE_LIST
#undef DEFINE_TY_VAL

#define DEFINE_CTY_VAL(CType, JSType) \
  inline bool SafeV8ConvertVal(Isolate* isolate, Local<Value> v, CType& outVal, Local<Value>& err, bool& hasError) { \
    if (v->Is##JSType()) { \
      outVal = v->JSType##Value(isolate->GetCurrentContext()).ToChecked(); \
      hasError = false; \
      return true; \
    } else { \
      MaybeLocal<String> mErrMsg =  \
        v8::String::NewFromUtf8(isolate, "Invalid type", v8::String::NewStringType::kNormalString); \
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked()); \
      hasError = true; \
      return false; \
    } \
  } \
  inline bool SafeV8CoerceVal(Isolate* isolate, Local<Value> v, CType& outVal, Local<Value>& err, bool& hasError) { \
    Maybe<CType> mv = v->JSType##Value(isolate->GetCurrentContext()); \
    if (mv.IsJust()) { \
      outVal = mv.FromJust(); \
      hasError = false; \
      return true; \
    } else { \
      MaybeLocal<String> mErrMsg =  \
        v8::String::NewFromUtf8(isolate, "Invalid type", v8::String::NewStringType::kNormalString); \
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked()); \
      hasError = true; \
      return false; \
    } \
  }

  DEFINE_CTY_VAL(bool, Boolean)
  DEFINE_CTY_VAL(double, Number)
  DEFINE_CTY_VAL(uint32_t, Uint32)
  DEFINE_CTY_VAL(int32_t, Int32)
#undef DEFINE_CTY_VAL

    //int64 version
  inline bool SafeV8ConvertVal(Isolate* isolate, Local<Value> v, int64_t& outVal, Local<Value>& err, bool& hasError) {
    if (v->IsNumber() && v->IntegerValue(isolate->GetCurrentContext()).To(&outVal)) {
      hasError = false;
      return true;
    }
    else {
      MaybeLocal<String> mErrMsg =
        v8::String::NewFromUtf8(isolate, "Invalid type", v8::String::NewStringType::kNormalString);
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked());
      hasError = true;
      return false;
    }
  }

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Common Base class definition
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

  /* Base class from which the various output classes derive from */
  class SafeV8Promise_Base
  {
  protected:
    Local<Value> err;
    bool exceptionThrown = false;
  public:
    SafeV8Promise_Base() {}

    SafeV8Promise_Base(bool _exceptionThrown, Local<Value> _err) : err(_err), exceptionThrown(_exceptionThrown) {}

    Local<Value> GetException()
    {
      return err;
    }
    bool GetIsExceptionThrown()
    {
      return exceptionThrown;
    }
  };

  const SafeV8Promise_Base Done;

  inline SafeV8Promise_Base safeV8Err(Local<Value> err)
  {
    SafeV8Promise_Base e(true, err);
    return e;
  }

  inline SafeV8Promise_Base safeV8Err(Isolate* isolate, char const * err)
  {
    MaybeLocal<String> mErrMsg = v8::String::NewFromUtf8(isolate, err, v8::String::NewStringType::kNormalString);
    SafeV8Promise_Base e(true, v8::Exception::TypeError(mErrMsg.ToLocalChecked()));
    return e;
  }

  inline SafeV8Promise_Base safeV8Err(Isolate* isolate, char const * err, v8::Local<v8::Value>(*errorType)(v8::Local<v8::String>))
  {
    MaybeLocal<String> mErrMsg = v8::String::NewFromUtf8(isolate, err, v8::String::NewStringType::kNormalString);
    SafeV8Promise_Base e(true, errorType(mErrMsg.ToLocalChecked()));
    return e;
  }

  inline v8::Local<v8::Value> v8Err(Isolate* isolate, char const * err, v8::Local<v8::Value>(*errorType)(v8::Local<v8::String>))
  {
    MaybeLocal<String> mErrMsg = v8::String::NewFromUtf8(isolate, err, v8::String::NewStringType::kNormalString);
    auto ret = errorType(mErrMsg.ToLocalChecked());
    return ret;
  }

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Monadic function structure for Convert API (i.e. if a JS object points to an array, attempt to convert the type to an array)
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////


  /* Class which handles the single output value case */
  class SafeV8Promise_GetOutput1 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
  public:
    SafeV8Promise_GetOutput1(Isolate* _isolate, Local<Value> _v1) : isolate(_isolate), v1(_v1) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput1>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        func(obj1);
        return *this;
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput1>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        SafeV8Promise_Base nestedCall = func(obj1);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
        return *this;
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput1>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  /* Class which handles 2 output value case */
  class SafeV8Promise_GetOutput2 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
    Local<Value> v2;
  public:
    SafeV8Promise_GetOutput2(Isolate* _isolate, Local<Value> _v1, Local<Value> _v2) : isolate(_isolate), v1(_v1), v2(_v2) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput2>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          func(obj1, obj2);
          return *this;
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput2>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          SafeV8Promise_Base nestedCall = func(obj1, obj2);
          exceptionThrown = nestedCall.GetIsExceptionThrown();
          err = nestedCall.GetException();
          return *this;
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput2>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  /* Class which handles 3 output value case */
  class SafeV8Promise_GetOutput3 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
    Local<Value> v2;
    Local<Value> v3;
  public:
    SafeV8Promise_GetOutput3(Isolate* _isolate, Local<Value> _v1, Local<Value> _v2, Local<Value> _v3) : isolate(_isolate), v1(_v1), v2(_v2), v3(_v3) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput3>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            func(obj1, obj2, obj3);
            return *this;
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput3>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            SafeV8Promise_Base nestedCall = func(obj1, obj2, obj3);
            exceptionThrown = nestedCall.GetIsExceptionThrown();
            err = nestedCall.GetException();
            return *this;
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput3>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

    /* Class which handles 4 output value case */
  class SafeV8Promise_GetOutput4 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
    Local<Value> v2;
    Local<Value> v3;
    Local<Value> v4;
  public:
    SafeV8Promise_GetOutput4(Isolate* _isolate, Local<Value> _v1, Local<Value> _v2, Local<Value> _v3, Local<Value> _v4) : isolate(_isolate), v1(_v1), v2(_v2), v3(_v3), v4(_v4) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput4>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      fourth_argument<F> obj4;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            if (SafeV8ConvertVal(isolate, v4, obj4, err, exceptionThrown))
            {
              func(obj1, obj2, obj3, obj4);
              return *this;
            }
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput4>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      fourth_argument<F> obj4;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            if (SafeV8ConvertVal(isolate, v4, obj4, err, exceptionThrown))
            {
              SafeV8Promise_Base nestedCall = func(obj1, obj2, obj3, obj4);
              exceptionThrown = nestedCall.GetIsExceptionThrown();
              err = nestedCall.GetException();
              return *this;
            }
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput4>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  /* Class which handles 5 output value case */
  class SafeV8Promise_GetOutput5 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
    Local<Value> v2;
    Local<Value> v3;
    Local<Value> v4;
    Local<Value> v5;
  public:
    SafeV8Promise_GetOutput5(Isolate* _isolate, Local<Value> _v1, Local<Value> _v2, Local<Value> _v3, Local<Value> _v4, Local<Value> _v5) : isolate(_isolate), v1(_v1), v2(_v2), v3(_v3), v4(_v4), v5(_v5) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput5>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      fourth_argument<F> obj4;
      fifth_argument<F> obj5;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            if (SafeV8ConvertVal(isolate, v4, obj4, err, exceptionThrown))
            {
              if (SafeV8ConvertVal(isolate, v5, obj5, err, exceptionThrown))
              {
                func(obj1, obj2, obj3, obj4, obj5);
                return *this;
              }
            }
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput5>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      fourth_argument<F> obj4;
      fifth_argument<F> obj5;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            if (SafeV8ConvertVal(isolate, v4, obj4, err, exceptionThrown))
            {
              if (SafeV8ConvertVal(isolate, v5, obj5, err, exceptionThrown))
              {
                SafeV8Promise_Base nestedCall = func(obj1, obj2, obj3, obj4, obj5);
                exceptionThrown = nestedCall.GetIsExceptionThrown();
                err = nestedCall.GetException();
                return *this;
              }
            }
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput5>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  /* Class which handles 6 output value case */
  class SafeV8Promise_GetOutput6 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
    Local<Value> v2;
    Local<Value> v3;
    Local<Value> v4;
    Local<Value> v5;
    Local<Value> v6;
  public:
    SafeV8Promise_GetOutput6(Isolate* _isolate, Local<Value> _v1, Local<Value> _v2, Local<Value> _v3, Local<Value> _v4, Local<Value> _v5, Local<Value> _v6) : isolate(_isolate), v1(_v1), v2(_v2), v3(_v3), v4(_v4), v5(_v5), v6(_v6) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput6>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      fourth_argument<F> obj4;
      fifth_argument<F> obj5;
      sixth_argument<F> obj6;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            if (SafeV8ConvertVal(isolate, v4, obj4, err, exceptionThrown))
            {
              if (SafeV8ConvertVal(isolate, v5, obj5, err, exceptionThrown))
              {
                if (SafeV8ConvertVal(isolate, v6, obj6, err, exceptionThrown))
                {
                  func(obj1, obj2, obj3, obj4, obj5, obj6);
                  return *this;
                }
              }
            }
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput6>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      second_argument<F> obj2;
      third_argument<F> obj3;
      fourth_argument<F> obj4;
      fifth_argument<F> obj5;
      sixth_argument<F> obj6;
      if (SafeV8ConvertVal(isolate, v1, obj1, err, exceptionThrown))
      {
        if (SafeV8ConvertVal(isolate, v2, obj2, err, exceptionThrown))
        {
          if (SafeV8ConvertVal(isolate, v3, obj3, err, exceptionThrown))
          {
            if (SafeV8ConvertVal(isolate, v4, obj4, err, exceptionThrown))
            {
              if (SafeV8ConvertVal(isolate, v5, obj5, err, exceptionThrown))
              {
                if (SafeV8ConvertVal(isolate, v6, obj6, err, exceptionThrown))
                {
                  SafeV8Promise_Base nestedCall = func(obj1, obj2, obj3, obj4, obj5, obj6);
                  exceptionThrown = nestedCall.GetIsExceptionThrown();
                  err = nestedCall.GetException();
                  return *this;
                }
              }
            }
          }
        }
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput6>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  /* Entry point to users who want to use the SafeV8 api */

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput1 marshal(Isolate* isolate, Local<Value> first)
  {
    return SafeV8Promise_GetOutput1(isolate, first);
  }

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput2 marshal(Isolate* isolate, Local<Value> first, Local<Value> second)
  {
    return SafeV8Promise_GetOutput2(isolate, first, second);
  }

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput3 marshal(Isolate* isolate, Local<Value> first, Local<Value> second, Local<Value> third)
  {
    return SafeV8Promise_GetOutput3(isolate, first, second, third);
  }

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput4 marshal(Isolate* isolate, Local<Value> first, Local<Value> second, Local<Value> third, Local<Value> fourth)
  {
    return SafeV8Promise_GetOutput4(isolate, first, second, third, fourth);
  }

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput5 marshal(Isolate* isolate, Local<Value> first, Local<Value> second, Local<Value> third, Local<Value> fourth, Local<Value> fifth)
  {
    return SafeV8Promise_GetOutput5(isolate, first, second, third, fourth, fifth);
  }

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput6 marshal(Isolate* isolate, Local<Value> first, Local<Value> second, Local<Value> third, Local<Value> fourth, Local<Value> fifth, Local<Value> sixth)
  {
    return SafeV8Promise_GetOutput6(isolate, first, second, third, fourth, fifth, sixth);
  }

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Monadic function structure for
//    Get API (i.e. get a particular field from a json object)
//    Has API (i.e. check for a particular field from a json object)
//    Delete API (i.e. delete a particular field from a json object)
//    HasOwnProperty API (i.e. check for a particular field from a json object, without going up the inheritence tree)
//    GetOwnPropertyDescriptor API
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#define GetStyleAPI(apiname, originalApiname, apitype)\
  template<typename ObjectType, typename KeyType>\
  bool SafeV8##apiname(Isolate* isolate, ObjectType object, KeyType key, Local<apitype>& outVal, Local<Value>& err, bool& hasError) {\
    if (object->originalApiname(isolate->GetCurrentContext(), key).ToLocal(&outVal))\
    {\
      hasError = false;\
      return true;\
    }\
    else\
    {\
      MaybeLocal<String> mErrMsg =\
        v8::String::NewFromUtf8(isolate, "##apiname failed", v8::String::NewStringType::kNormalString);\
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked());\
      hasError = true;\
      return false;\
    }\
  }\
  \
  template<typename ObjectType, typename KeyType>\
  class SafeV8_##apiname##Output : public SafeV8Promise_Base\
  {\
  private:\
    Isolate* isolate;\
    ObjectType object;\
    KeyType key;\
  public:\
    SafeV8_##apiname##Output(Isolate* _isolate, ObjectType _object, KeyType _key) : isolate(_isolate), object(_object), key(_key) {}\
  \
    template<typename F>\
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8_##apiname##Output>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())\
    {\
      Local<Value> outVal;\
      if (SafeV8##apiname(isolate, object, key, outVal, err, exceptionThrown))\
      {\
        func(outVal);\
        return *this;\
      }\
  \
      if (!customException.IsEmpty())\
      {\
        err = customException;\
      }\
      return *this;\
    }\
  \
    template<typename F>\
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8_##apiname##Output>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())\
    {\
      Local<Value> outVal;\
      if (SafeV8##apiname(isolate, object, key, outVal, err, exceptionThrown))\
      {\
        SafeV8Promise_Base nestedCall = func(outVal);\
        exceptionThrown = nestedCall.GetIsExceptionThrown();\
        err = nestedCall.GetException();\
        return *this;\
      }\
  \
      if (!customException.IsEmpty())\
      {\
        err = customException;\
      }\
      return *this;\
    }\
  \
    template<typename F>\
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)\
    {\
      if (exceptionThrown)\
      {\
        func(err);\
      }\
    }\
  \
    template<typename F>\
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8_##apiname##Output>::type onFail(F func)\
    {\
      if (exceptionThrown)\
      {\
        SafeV8Promise_Base nestedCall = func(err);\
        exceptionThrown = nestedCall.GetIsExceptionThrown();\
        err = nestedCall.GetException();\
      }\
      return *this;\
    }\
  };\
  \
  template<typename ObjectType, typename KeyType>\
  V8_WARN_UNUSED_RESULT inline SafeV8_##apiname##Output<ObjectType, KeyType> apiname(Isolate* isolate, ObjectType object, KeyType key)\
  {\
    return SafeV8_##apiname##Output<ObjectType, KeyType>(isolate, object, key);\
  }

  GetStyleAPI(getProp, Get, Value)
  GetStyleAPI(hasProp, Has, bool)
  GetStyleAPI(delProp, Delete, bool)
  GetStyleAPI(hasOwnProp, HasOwnProperty, bool)
  GetStyleAPI(getOwnPropDescriptor, GetOwnPropertyDescriptor, Value)

#undef GetStyleAPI

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Monadic function structure for Set API (i.e. set a particular field from a json object)
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

  template<typename ObjectType, typename KeyType>
  bool SafeV8Set(Isolate* isolate, ObjectType object, KeyType key, Local<Value> val, Local<Value>& err, bool& hasError) {
    if (object->Set(isolate->GetCurrentContext(), key, val).FromMaybe(false))
    {
      hasError = false;
      return true;
    }
    else
    {
      MaybeLocal<String> mErrMsg =
        v8::String::NewFromUtf8(isolate, "Set failed", v8::String::NewStringType::kNormalString);
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked());
      hasError = true;
      return false;
    }
  }

  template<typename ObjectType, typename KeyType>
  class SafeV8_setPropOutput : public SafeV8Promise_Base
  {
  public:

    SafeV8_setPropOutput(Isolate* isolate, ObjectType object, KeyType key, Local<Value> val)
    {
      SafeV8Set(isolate, object, key, val, err, exceptionThrown);
    }

    SafeV8_setPropOutput(Local<Value> exception)
    {
      exceptionThrown = true;
      err = exception;
    }

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8_setPropOutput>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      if (!exceptionThrown)
      {
        func();
        return *this;
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8_setPropOutput>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      if (!exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func();
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
        return *this;
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8_setPropOutput>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  template<typename ObjectType, typename KeyType>
  V8_WARN_UNUSED_RESULT inline SafeV8_setPropOutput<ObjectType, KeyType> setProp(Isolate* isolate, ObjectType object, KeyType key, Local<Value> val)
  {
    return SafeV8_setPropOutput<ObjectType, KeyType>(isolate, object, key, val);
  }

  template<typename ObjectType, typename KeyType, typename GetObjectType, typename GetKeyType>
  V8_WARN_UNUSED_RESULT inline SafeV8_setPropOutput<ObjectType, KeyType> setProp(Isolate* isolate, ObjectType object, KeyType key, SafeV8_getPropOutput<GetObjectType, GetKeyType> val)
  {
    SafeV8_setPropOutput<ObjectType, KeyType>* ptr;

    val.onVal([&](Local<Value> result) {
      ptr = new SafeV8_setPropOutput<ObjectType, KeyType>(isolate, object, key, result);
    }).onFail([&](Local<Value> exception) {
      ptr = new SafeV8_setPropOutput<ObjectType, KeyType>(exception);
    });

    SafeV8_setPropOutput<ObjectType, KeyType> ret(*ptr);
    return ret;
  }

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Monadic function structure for
//    toString API (i.e. stringify object)
//    GetPropertyNames (i.e. get the list properties on this json object)
//    GetOwnPropertyNames (i.e. get the list properties on this json object, without going up the inheritence tree)
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#define ToStringStyleAPI(apiname, originalApiname, apitype, v8obj)\
  inline bool SafeV8##apiname(Isolate* isolate, Local<v8obj> v, Local<apitype>& outStringVal, Local<Value>& err, bool& hasError) {\
    MaybeLocal<apitype> ret = v->originalApiname(isolate->GetCurrentContext());\
    if (!ret.IsEmpty()) {\
      outStringVal = ret.FromMaybe(Local<apitype>());\
      hasError = false;\
      return true;\
    }\
    else {\
      MaybeLocal<String> mErrMsg =\
        v8::String::NewFromUtf8(isolate, "Could not convert to string", v8::String::NewStringType::kNormalString);\
      err = v8::Exception::TypeError(mErrMsg.ToLocalChecked());\
      hasError = true;\
      return false;\
    }\
  }\
\
  class SafeV8Promise_GetOutput_##apiname : public SafeV8Promise_Base\
  {\
  private:\
    Isolate* isolate;\
    Local<v8obj> v1;\
  public:\
    SafeV8Promise_GetOutput_##apiname(Isolate* _isolate, Local<v8obj> _v1) : isolate(_isolate), v1(_v1) {}\
\
    template<typename F>\
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput_##apiname>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())\
    {\
      first_argument<F> obj1;\
      if (SafeV8##apiname(isolate, v1, obj1, err, exceptionThrown))\
      {\
        func(obj1);\
        return *this;\
      }\
\
      if (!customException.IsEmpty())\
      {\
        err = customException;\
      }\
      return *this;\
    }\
\
    template<typename F>\
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput_##apiname>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())\
    {\
      first_argument<F> obj1;\
      if (SafeV8##apiname(isolate, v1, obj1, err, exceptionThrown))\
      {\
        SafeV8Promise_Base nestedCall = func(obj1);\
        exceptionThrown = nestedCall.GetIsExceptionThrown();\
        err = nestedCall.GetException();\
        return *this;\
      }\
\
      if (!customException.IsEmpty())\
      {\
        err = customException;\
      }\
      return *this;\
    }\
\
    template<typename F>\
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)\
    {\
      if (exceptionThrown)\
      {\
        func(err);\
      }\
    }\
\
    template<typename F>\
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput_##apiname>::type onFail(F func)\
    {\
      if (exceptionThrown)\
      {\
        SafeV8Promise_Base nestedCall = func(err);\
        exceptionThrown = nestedCall.GetIsExceptionThrown();\
        err = nestedCall.GetException();\
      }\
      return *this;\
    }\
  };\
\
\
  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput_##apiname apiname(Isolate* isolate, Local<v8obj> first) \
  {\
    return SafeV8Promise_GetOutput_##apiname(isolate, first); \
  }\


  ToStringStyleAPI(toString, ToString, String, Value)
  ToStringStyleAPI(getPropNames, GetPropertyNames, Array, Object)
  ToStringStyleAPI(getOwnPropNames, GetOwnPropertyNames, Array, Object)

#undef ToStringStyleAPI

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Monadic function structure for Unsafe casting APIs - such as converting to bool, double, uint32_t, int32_t
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

  class SafeV8Promise_GetOutput_Coerce1 : public SafeV8Promise_Base
  {
  private:
    Isolate* isolate;
    Local<Value> v1;
  public:
    SafeV8Promise_GetOutput_Coerce1(Isolate* _isolate, Local<Value> _v1) : isolate(_isolate), v1(_v1) {}

    //Returns the marshalled and converted values. The lambda provided does not marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_same<return_argument<F>, void>::value, SafeV8Promise_GetOutput_Coerce1>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      if (SafeV8CoerceVal(isolate, v1, obj1, err, exceptionThrown))
      {
        func(obj1);
        return *this;
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Returns the marshalled and converted values. The lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput_Coerce1>::type onVal(F func, v8::Local<v8::Value> customException = v8::Local<v8::Value>())
    {
      first_argument<F> obj1;
      if (SafeV8CoerceVal(isolate, v1, obj1, err, exceptionThrown))
      {
        SafeV8Promise_Base nestedCall = func(obj1);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
        return *this;
      }

      if (!customException.IsEmpty())
      {
        err = customException;
      }
      return *this;
    }

    //Handle any errors caught so far. The error handling lambda provided does not marshal additional values inside
    template<typename F>
    typename std::enable_if<std::is_same<return_argument<F>, void>::value, void>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        func(err);
      }
    }

    //Handle any errors caught so far. The error handling lambda provided does marshal additional values inside
    template<typename F>
    V8_WARN_UNUSED_RESULT typename std::enable_if<std::is_base_of<SafeV8Promise_Base, return_argument<F>>::value, SafeV8Promise_GetOutput_Coerce1>::type onFail(F func)
    {
      if (exceptionThrown)
      {
        SafeV8Promise_Base nestedCall = func(err);
        exceptionThrown = nestedCall.GetIsExceptionThrown();
        err = nestedCall.GetException();
      }
      return *this;
    }
  };

  V8_WARN_UNUSED_RESULT inline SafeV8Promise_GetOutput_Coerce1 implicitCoerce(Isolate* isolate, Local<Value> first)
  {
    return SafeV8Promise_GetOutput_Coerce1(isolate, first);
  }

}
#endif  // SAFE_V8_H_