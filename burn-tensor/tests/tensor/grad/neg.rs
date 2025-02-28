use crate::tensor::TestADTensor;
use burn_tensor::Data;

#[test]
fn should_diff_neg() {
    let data_1 = Data::<f32, 2>::from([[1.0, 7.0], [2.0, 3.0]]);
    let data_2 = Data::<f32, 2>::from([[4.0, 7.0], [2.0, 3.0]]);

    let tensor_1 = TestADTensor::from_data(data_1);
    let tensor_2 = TestADTensor::from_data(data_2);

    let tensor_3 = tensor_1.matmul(&tensor_2.neg());
    let tensor_4 = tensor_3.neg();
    let grads = tensor_4.backward();

    let grad_1 = tensor_1.grad(&grads).unwrap();
    let grad_2 = tensor_2.grad(&grads).unwrap();

    assert_eq!(grad_1.to_data(), Data::from([[11.0, 5.0], [11.0, 5.0]]));
    assert_eq!(grad_2.to_data(), Data::from([[3.0, 3.0], [10.0, 10.0]]));
}
